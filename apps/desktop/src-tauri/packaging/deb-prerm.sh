#!/bin/sh
# prerm of the ubiqX package. Before the files go, the app's own executable takes off
# what this product applied to the system (the hosts block, the family resolver, the
# browser policies), keeping what the other product on this machine still wants.
#
# Only on "remove": dpkg also runs prerm with "upgrade" on every update of the package,
# and an update must not take the protection off. A removal that fails here never stops
# the package removal; REMOVAL.md, in the module's folder under docs/ of the source tree,
# has the manual steps (packaging files stay neutral, so the folder is not named here).
set -e

case "$1" in
  remove)
    for exe in $(dpkg-query -L "${DPKG_MAINTSCRIPT_PACKAGE:-ubiqx}" 2>/dev/null | grep '^/usr/bin/'); do
      if [ -f "$exe" ] && [ -x "$exe" ]; then
        "$exe" --shield-helper remove --owner ubiqx --scope all || true
        break
      fi
    done
    ;;
esac

exit 0
