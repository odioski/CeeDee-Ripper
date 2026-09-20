"""Exercise installer routing without installing anything: python3 scripts/test-install-deps.py."""
import pathlib
import shlex
import subprocess
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("install-deps.sh")


def bash(code):
    return subprocess.run(
        ["bash", "-c", f"source {shlex.quote(str(SCRIPT))}\n{code}"],
        text=True, capture_output=True,
    )


class InstallerTests(unittest.TestCase):
    def test_distribution_detection(self):
        for distro, like, expected in [
            ("fedora", "", "fedora"), ("ubuntu", "debian", "debian"),
            ("debian", "", "debian"), ("kali", "", "debian"),
            ("slackware", "", "slackware"),
            ("slax", "debian", "debian"), ("slax", "slackware", "slackware"),
            ("arch", "", "arch"), ("opensuse-tumbleweed", "suse", "suse"),
        ]:
            with self.subTest(distro=distro, like=like), tempfile.NamedTemporaryFile(mode="w") as f:
                f.write(f'ID={distro}\nID_LIKE="{like}"\n')
                f.flush()
                result = bash(f'detect_family {shlex.quote(f.name)}')
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), expected)

    def test_slax_without_id_like(self):
        with tempfile.NamedTemporaryFile(mode="w") as f:
            f.write("ID=slax\n")
            f.flush()
            for manager, expected in [("apt-get", "debian"), ("slackpkg", "slackware")]:
                result = bash(f'have() {{ [[ $1 == {manager} ]]; }}\ndetect_family {shlex.quote(f.name)}')
                self.assertEqual(result.stdout.strip(), expected)

    def test_unknown_distribution_fails(self):
        with tempfile.NamedTemporaryFile(mode="w") as f:
            f.write("ID=unsupported\n")
            f.flush()
            self.assertNotEqual(bash(f'detect_family {shlex.quote(f.name)}').returncode, 0)

    def test_apt_mapping_and_order(self):
        result = bash('''
detect_family() { echo debian; }
dpkg-query() { return 1; }
as_root() { printf 'RUN %s\n' "$*"; }
main
''')
        self.assertEqual(result.returncode, 0, result.stderr)
        commands = [line for line in result.stdout.splitlines() if line.startswith("RUN ")]
        self.assertEqual(commands[0], "RUN apt-get update")
        self.assertIn("apt-get install -y", commands[1])
        packages = commands[1].split()
        self.assertIn("libgtk-4-dev", packages)
        self.assertIn("clang-tools", packages)
        self.assertIn("libcurl4-openssl-dev", packages)
        self.assertNotIn("libcurl4", packages)
        self.assertNotIn("libglib2.0-0", packages)

    def test_fedora_mapping_and_dnf5(self):
        result = bash('''
detect_family() { echo fedora; }
have() { [[ $1 == dnf5 ]]; }
rpm() { return 1; }
as_root() { printf 'RUN %s\n' "$*"; }
main
''')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("RUN dnf5 install -y", result.stdout)
        for package in ["gtk4-devel", "clang-tools-extra", "gcc-c++", "cmake-extras"]:
            self.assertIn(package, result.stdout)

    def test_installed_packages_do_not_trigger_install(self):
        for family, query in [("debian", "dpkg-query() { printf 'ii '; }"),
                              ("fedora", "rpm() { return 0; }")]:
            result = bash(f'''
detect_family() {{ echo {family}; }}
dnf_command() {{ echo dnf; }}
{query}
as_root() {{ echo UNEXPECTED_INSTALL; return 99; }}
main
''')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("UNEXPECTED_INSTALL", result.stdout)

    def test_install_failure_propagates(self):
        result = bash('''
detect_family() { echo debian; }
dpkg-query() { return 1; }
as_root() { return 42; }
main
''')
        self.assertEqual(result.returncode, 42)
        self.assertNotIn("Done.", result.stdout)

    def test_slackware_exact_package_matching(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ["gcc-g++-12.2.0-x86_64-1", "gtk4-4.10.0-x86_64-1"]:
                pathlib.Path(directory, name).touch()
            quoted = shlex.quote(directory)
            self.assertEqual(bash(f'slackware_installed gtk4 {quoted}').returncode, 0)
            self.assertEqual(bash(f'slackware_installed gcc-g++ {quoted}').returncode, 0)
            self.assertNotEqual(bash(f'slackware_installed gcc {quoted}').returncode, 0)

    def test_slackware_does_not_silently_skip_missing_packages(self):
        result = bash('''
have() { return 0; }
slackware_installed() { return 1; }
as_root() { printf 'RUN %s\n' "$*"; }
install_slackware
''')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("RUN slackpkg update", result.stdout)
        self.assertIn("slackpkg -batch=on -default_answer=y install", result.stdout)
        self.assertIn("llvm", result.stdout)
        self.assertIn("Dependencies still missing", result.stderr)
        self.assertNotIn("Done.", result.stdout)

    def test_cli_help_and_invalid_options(self):
        for args, status in [(["--help"], 0), (["-h"], 0), (["--bogus"], 2),
                             (["-R", "extra"], 2)]:
            result = subprocess.run(["bash", str(SCRIPT), *args], capture_output=True, text=True)
            self.assertEqual(result.returncode, status, result.stderr)

    def test_tracking_install_repeat_partial_failure_and_removal(self):
        with tempfile.TemporaryDirectory() as directory:
            state = pathlib.Path(directory)
            inventory = state / "inventory"
            inventory.write_text("existing\n")
            setup = f"""
state={shlex.quote(directory)}
installed_packages() {{ cat "$state/inventory"; }}
"""
            # Include a dependency not explicitly listed in the install command.
            result = bash(setup + """
main() { printf 'dependency\nexisting\nrequested\n' > "$state/inventory"; }
tracked_action debian install "$state"
""")
            self.assertEqual(result.returncode, 0, result.stderr)
            ledger = state / "debian.packages"
            self.assertEqual(ledger.read_text(), "dependency\nrequested\n")
            result = bash(setup + 'main() { :; }; tracked_action debian install "$state"')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(ledger.read_text(), "dependency\nrequested\n")
            result = bash(setup + """
main() { printf 'dependency\nexisting\npartial\nrequested\n' > "$state/inventory"; return 42; }
tracked_action debian install "$state"
""")
            self.assertEqual(result.returncode, 42, result.stderr)
            self.assertEqual(ledger.read_text(), "dependency\npartial\nrequested\n")
            # A partial removal failure retains exactly the packages still present.
            result = bash(setup + """
remove_packages() {
  printf '%s\n' "$@" > "$state/targets"
  printf 'existing\npartial\n' > "$state/inventory"
  return 7
}
tracked_action debian remove "$state"
""")
            self.assertEqual(result.returncode, 7, result.stderr)
            self.assertEqual((state / "targets").read_text(), "debian\ndependency\npartial\nrequested\n")
            self.assertEqual(ledger.read_text(), "partial\n")
            result = bash(setup + """
remove_packages() { printf 'existing\n' > "$state/inventory"; }
tracked_action debian remove "$state"
""")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(ledger.read_text(), "")

    def test_untracked_removal_is_noop(self):
        with tempfile.TemporaryDirectory() as directory:
            result = bash(f"""
installed_packages() {{ echo existing; }}
remove_packages() {{ echo UNEXPECTED_REMOVAL; return 99; }}
tracked_action debian remove {shlex.quote(directory)}
""")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("UNEXPECTED_REMOVAL", result.stdout)
            self.assertIn("No recorded", result.stdout)

    def test_removal_backends(self):
        for family, command in [("debian", "apt-get"), ("fedora", "dnf5"),
                                ("arch", "pacman"), ("suse", "zypper"),
                                ("slackware", "removepkg")]:
            with self.subTest(family=family):
                result = bash(f"""
dnf_command() {{ echo dnf5; }}
{command}() {{ printf '%s\n' "$@"; }}
remove_packages {family} sample-package <<< y
""")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("sample-package", result.stdout.splitlines())
                self.assertNotIn("-y", result.stdout.splitlines())
                self.assertNotIn("autoremove", result.stdout)

    def test_debian_inventory_keeps_held_and_partial_packages(self):
        result = bash("""
dpkg-query() { printf 'ii  installed:amd64\nhi  held:amd64\niU  unpacked:amd64\nrc  removed:amd64\n'; }
installed_packages debian
""")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "held:amd64\ninstalled:amd64\nunpacked:amd64\n")


if __name__ == "__main__":
    unittest.main()
