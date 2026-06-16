Name:           ceedee-ripper
Version:        1.1.0
Release:        1%{?dist}
Summary:        Linux desktop app for extracting audio CDs

License:        MIT
URL:            https://github.com/odioski/CeeDee-Ripper
Source0:        %{url}/archive/refs/tags/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  clang-devel
BuildRequires:  pkgconfig
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib
BuildRequires:  pkgconfig(gstreamer-1.0)
BuildRequires:  pkgconfig(gstreamer-audio-1.0)
BuildRequires:  pkgconfig(gstreamer-plugins-base-1.0)
BuildRequires:  pkgconfig(libdiscid)
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)

Requires:       cdparanoia
Requires:       cd-discid
Requires:       eject
Requires:       flac
Requires:       gstreamer1-plugins-base
Requires:       gstreamer1-plugins-good
Requires:       gstreamer1-plugins-ugly-free
Requires:       lame
Requires:       libdiscid
Requires:       vorbis-tools

%description
CeeDee Ripper detects audio CDs, looks up metadata, and extracts selected
tracks to FLAC, MP3, WAV, or Ogg Vorbis files.

%prep
%autosetup -n CeeDee-Ripper-%{version}

%build
cargo build --release --locked --features "gtk-ui egui-ui"

%check
cargo test --locked --features "gtk-ui egui-ui"
desktop-file-validate resources/ceedee-ripper.desktop
appstream-util validate-relax --nonet resources/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml

%install
install -Dm0755 target/release/ceedee-ripper %{buildroot}%{_bindir}/ceedee-ripper
install -Dm0644 resources/ceedee-ripper.desktop %{buildroot}%{_datadir}/applications/ceedee-ripper.desktop
install -Dm0644 resources/images/ceedee-ripper.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/io.github.odioski.ceedee_ripper.png
install -Dm0644 resources/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml %{buildroot}%{_datadir}/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml
install -Dm0644 LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE

%files
%license %{_datadir}/licenses/%{name}/LICENSE
%{_bindir}/ceedee-ripper
%{_datadir}/applications/ceedee-ripper.desktop
%{_datadir}/icons/hicolor/256x256/apps/io.github.odioski.ceedee_ripper.png
%{_datadir}/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml

%changelog
* Fri May 08 2026 Omar Daniels <odioski@users.noreply.github.com> - 1.1.0-1
- Initial Fedora/COPR recipe.
