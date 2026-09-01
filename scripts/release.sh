#!/bin/bash
# ============================================================
#   AegisPanel OS – Release Wizard
#   Interaktiver CLI-Assistent zum Erstellen neuer Versionen,
#   Bauen von Images & automatischem GitHub Release Upload
# ============================================================
set -euo pipefail

# ── Farben & Symbole ──────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
DIM='\033[2m'
RESET='\033[0m'

OK="  ${GREEN}✓${RESET}"
ERR="  ${RED}✗${RESET}"
ARROW="  ${CYAN}▶${RESET}"
WARN="  ${YELLOW}⚠${RESET}"
STEP="  ${MAGENTA}◆${RESET}"

PROJECT_DIR="/home/oliver/dev/panel"
BUILDROOT_DIR="${PROJECT_DIR}/buildroot"
IMAGES_DIR="${BUILDROOT_DIR}/output/images"
CORE_TOML="${PROJECT_DIR}/src/aegispanel-core/Cargo.toml"
CORE_MK="${PROJECT_DIR}/packages/aegispanel-core/aegispanel-core.mk"
HOST_CARGO="${BUILDROOT_DIR}/output/host/bin/cargo"

SELECTED_BUILD_TYPE="1"
SELECTED_NEW_VERSION="1.0.0"
CHANGELOG_MSG=""

# ── Hilfsfunktionen ───────────────────────────────────────
banner() {
    clear
    echo ""
    echo -e "${BOLD}${BLUE}╔══════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}${BLUE}║       AegisPanel OS  •  Release Wizard           ║${RESET}"
    echo -e "${BOLD}${BLUE}╚══════════════════════════════════════════════════╝${RESET}"
    echo ""
}

step_header() {
    echo ""
    echo -e "${BOLD}${CYAN}━━━  $1  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
}

confirm() {
    local msg="$1"
    local default="${2:-y}"
    if [[ "$default" == "y" ]]; then
        local prompt="[Y/n]"
    else
        local prompt="[y/N]"
    fi
    echo -en "${ARROW} ${BOLD}${msg}${RESET} ${DIM}${prompt}${RESET} "
    read -r answer
    answer="${answer:-$default}"
    [[ "${answer,,}" == "y" ]]
}

ask() {
    local msg="$1"
    local default="${2:-}"
    if [[ -n "$default" ]]; then
        echo -en "${ARROW} ${BOLD}${msg}${RESET} ${DIM}[${default}]${RESET}: "
    else
        echo -en "${ARROW} ${BOLD}${msg}${RESET}: "
    fi
    read -r answer
    echo "${answer:-$default}"
}

run_cmd() {
    local desc="$1"
    shift
    echo -en "${ARROW} ${desc}... "
    if "$@" >> /tmp/aegispanel_release.log 2>&1; then
        echo -e "${OK}"
    else
        echo -e "${ERR}"
        echo ""
        echo -e "${WARN} Fehler aufgetreten! Log:"
        tail -20 /tmp/aegispanel_release.log
        exit 1
    fi
}

run_cmd_live() {
    local desc="$1"
    shift
    echo -e "${ARROW} ${BOLD}${desc}${RESET}"
    echo ""
    "$@"
    echo ""
}

check_prereqs() {
    step_header "Voraussetzungen prüfen"

    local ok=true

    echo -en "${ARROW} Buildroot vorhanden... "
    if [[ -d "${BUILDROOT_DIR}" ]]; then
        echo -e "${OK}"
    else
        echo -e "${ERR} ${BUILDROOT_DIR} nicht gefunden"
        ok=false
    fi

    echo -en "${ARROW} Host-Toolchain vorhanden... "
    if [[ -f "${BUILDROOT_DIR}/output/host/bin/gcc" ]]; then
        echo -e "${OK}"
    else
        echo -e "${WARN} Toolchain fehlt – erster Build dauert länger"
    fi

    echo -en "${ARROW} Overlay-Symlink vorhanden... "
    local overlay_link="${BUILDROOT_DIR}/board/raspberrypi/zero2w/overlays/aegispanel-overlay"
    if [[ -L "${overlay_link}" ]]; then
        echo -e "${OK}"
    else
        echo -en "${WARN} Fehlend – wird jetzt erstellt... "
        mkdir -p "${BUILDROOT_DIR}/board/raspberrypi/zero2w/overlays"
        ln -sf "${PROJECT_DIR}/board/raspberrypi/zero2w/overlays/aegispanel-overlay" "${overlay_link}"
        echo -e "${OK}"
    fi

    echo -en "${ARROW} create_sdcard.sh vorhanden... "
    if [[ -f "${PROJECT_DIR}/scripts/create_sdcard.sh" ]]; then
        echo -e "${OK}"
    else
        echo -e "${ERR} scripts/create_sdcard.sh fehlt"
        ok=false
    fi

    if [[ "$ok" != "true" ]]; then
        echo ""
        echo -e "${ERR} Nicht alle Voraussetzungen erfüllt. Abbruch."
        exit 1
    fi
}

get_current_version() {
    grep '^version' "${CORE_TOML}" | head -1 | sed 's/.*"\(.*\)"/\1/'
}

bump_version() {
    local current="$1"
    local type="$2"
    IFS='.' read -ra parts <<< "$current"
    local major="${parts[0]}"
    local minor="${parts[1]}"
    local patch="${parts[2]}"
    case "$type" in
        major) echo "$((major + 1)).0.0" ;;
        minor) echo "${major}.$((minor + 1)).0" ;;
        patch) echo "${major}.${minor}.$((patch + 1))" ;;
    esac
}

select_build_type() {
    step_header "Build-Typ wählen"

    echo -e "  ${DIM}Was hast du geändert?${RESET}"
    echo ""
    echo -e "  ${CYAN}1)${RESET}  ${BOLD}Rust-Code${RESET}          ${DIM}(src/aegispanel-core/  geändert)${RESET}"
    echo -e "  ${CYAN}2)${RESET}  ${BOLD}Overlay / Config${RESET}   ${DIM}(board/overlays/, systemd-Services geändert)${RESET}"
    echo -e "  ${CYAN}3)${RESET}  ${BOLD}Pakete / Kernel${RESET}    ${DIM}(defconfig, neue Buildroot-Pakete geändert)${RESET}"
    echo -e "  ${CYAN}4)${RESET}  ${BOLD}Vollständig${RESET}        ${DIM}(Clean-Build, Major-Version)${RESET}"
    echo ""
    echo -en "${ARROW} ${BOLD}Auswahl${RESET} ${DIM}[1-4]${RESET}: "
    read -r choice
    SELECTED_BUILD_TYPE="${choice:-1}"
}

version_wizard() {
    step_header "Versions-Management"

    local current
    current=$(get_current_version)
    echo -e "${ARROW} Aktuelle Version: ${BOLD}${YELLOW}v${current}${RESET}"
    echo ""

    echo -e "  ${CYAN}1)${RESET}  Patch-Update     ${DIM}v${current} → v$(bump_version "$current" patch)${RESET}   ${DIM}(Bugfixes)${RESET}"
    echo -e "  ${CYAN}2)${RESET}  Minor-Update     ${DIM}v${current} → v$(bump_version "$current" minor)${RESET}   ${DIM}(Neue Features)${RESET}"
    echo -e "  ${CYAN}3)${RESET}  Major-Update     ${DIM}v${current} → v$(bump_version "$current" major)${RESET}   ${DIM}(Breaking Changes)${RESET}"
    echo -e "  ${CYAN}4)${RESET}  Manuell eingeben"
    echo -e "  ${CYAN}5)${RESET}  Version beibehalten ${DIM}(v${current})${RESET}"
    echo ""
    echo -en "${ARROW} ${BOLD}Auswahl${RESET} ${DIM}[1-5]${RESET}: "
    read -r vchoice

    local new_version
    case "$vchoice" in
        1) new_version=$(bump_version "$current" patch) ;;
        2) new_version=$(bump_version "$current" minor) ;;
        3) new_version=$(bump_version "$current" major) ;;
        4) new_version=$(ask "Neue Version eingeben" "$current") ;;
        5) new_version="$current" ;;
        *) new_version="$current" ;;
    esac

    if [[ "$new_version" != "$current" ]]; then
        echo ""
        echo -e "${ARROW} Setze Version auf ${BOLD}${GREEN}v${new_version}${RESET}..."

        # Cargo.toml
        sed -i "s/^version = \"${current}\"/version = \"${new_version}\"/" "${CORE_TOML}"
        # aegispanel-core.mk
        sed -i "s/^AEGISPANEL_CORE_VERSION = .*/AEGISPANEL_CORE_VERSION = ${new_version}/" "${CORE_MK}"

        echo -e "${OK} Version auf v${new_version} gesetzt"
    fi

    SELECTED_NEW_VERSION="$new_version"
}

build_rust_only() {
    local version="$1"
    step_header "Build: Rust-Daemon aegispanel-core v${version}"

    if [[ -f "${HOST_CARGO}" ]]; then
        run_cmd "Cargo.lock aktualisieren" \
            bash -c "cd ${PROJECT_DIR}/src/aegispanel-core && ${HOST_CARGO} generate-lockfile"
    fi

    run_cmd "Alten Build-Cache löschen" \
        bash -c "rm -rf ${BUILDROOT_DIR}/output/build/aegispanel-core-*"

    echo ""
    run_cmd_live "Cross-Kompilierung starten" \
        bash -c "cd ${BUILDROOT_DIR} && FORCE_UNSAFE_CONFIGURE=1 make aegispanel-core -j$(nproc)"
}

build_overlay_only() {
    step_header "Build: Overlay & Root-Filesystem"

    run_cmd_live "target-finalize + rootfs-ext2 neu erstellen" \
        bash -c "cd ${BUILDROOT_DIR} && FORCE_UNSAFE_CONFIGURE=1 make target-finalize rootfs-ext2 -j$(nproc)"
}

build_full_incremental() {
    step_header "Build: Inkrementell (alle geänderten Pakete)"

    run_cmd_live "Buildroot make -j$(nproc)" \
        bash -c "cd ${BUILDROOT_DIR} && FORCE_UNSAFE_CONFIGURE=1 make -j$(nproc)"
}

build_clean() {
    step_header "Build: Vollständiger Clean-Build"

    echo -e "${WARN} ${BOLD}${RED}WARNUNG:${RESET} Alle Build-Artefakte werden gelöscht!"
    echo -e "${DIM}   Die Toolchain (output/host/) bleibt erhalten.${RESET}"
    echo ""

    if ! confirm "Wirklich fortfahren?"; then
        echo -e "${WARN} Abgebrochen."
        exit 0
    fi

    run_cmd "Build-Ordner bereinigen (ohne Toolchain)" \
        bash -c "rm -rf ${BUILDROOT_DIR}/output/build ${BUILDROOT_DIR}/output/target ${BUILDROOT_DIR}/output/images ${BUILDROOT_DIR}/output/staging"

    run_cmd "Overlay-Symlink neu setzen" \
        bash -c "mkdir -p ${BUILDROOT_DIR}/board/raspberrypi/zero2w/overlays && \
                 ln -sf ${PROJECT_DIR}/board/raspberrypi/zero2w/overlays/aegispanel-overlay \
                        ${BUILDROOT_DIR}/board/raspberrypi/zero2w/overlays/aegispanel-overlay"

    run_cmd_live "Vollständiger Build" \
        bash -c "cd ${PROJECT_DIR} && FORCE_UNSAFE_CONFIGURE=1 ./scripts/build.sh"
}

build_sdcard() {
    step_header "SD-Card-Image erstellen"

    run_cmd_live "sdcard.img zusammenbauen" \
        bash "${PROJECT_DIR}/scripts/create_sdcard.sh"
}

github_release_wizard() {
    local version="$1"
    local notes="$2"
    step_header "GitHub Release & Repository Sync"

    cd "${PROJECT_DIR}"
    git config --global --add safe.directory "${PROJECT_DIR}" 2>/dev/null || true
    git config --global user.name "AegisPanel Builder" 2>/dev/null || true
    git config --global user.email "build@aegispanel.internal" 2>/dev/null || true

    # 1. Check/Init Git
    if [[ ! -d ".git" ]]; then
        echo -en "${ARROW} Git Repository initialisieren... "
        git init -b main >> /tmp/aegispanel_release.log 2>&1
        echo -e "${OK}"
    fi

    # 2. Check Git Remote
    local repo_url="https://github.com/moinmoin-64/aegispanel.git"
    git remote remove origin 2>/dev/null || true
    git remote add origin "$repo_url" 2>/dev/null || true
    echo -e "${ARROW} Remote Repository: ${BOLD}${CYAN}${repo_url}${RESET}"

    # 3. GitHub Token abfragen für sicheren Push & Release
    local token="${GITHUB_TOKEN:-}"
    if [[ -z "$token" ]]; then
        echo ""
        echo -e "${WARN} ${BOLD}GitHub Authentifizierung:${RESET}"
        echo -e "${DIM}  Da GitHub keine normalen Passwörter mehr akzeptiert, benötigst du ein${RESET}"
        echo -e "${DIM}  Personal Access Token (PAT mit 'repo' Rechten) von: https://github.com/settings/tokens${RESET}"
        echo ""
        token=$(ask "GitHub Personal Access Token (PAT)" "")
    fi

    local auth_push_url="$repo_url"
    if [[ -n "$token" ]]; then
        auth_push_url="https://oauth2:${token}@github.com/moinmoin-64/aegispanel.git"
    fi

    # 4. Git Add & Commit
    echo -en "${ARROW} Änderungen stagen und committen... "
    git add -A >> /tmp/aegispanel_release.log 2>&1
    local commit_msg="Release v${version}: ${notes:-"Update v${version}"}"
    git commit -m "$commit_msg" >> /tmp/aegispanel_release.log 2>&1 || true
    echo -e "${OK}"

    # 5. Tag erstellen
    echo -en "${ARROW} Git Tag v${version} setzen... "
    git tag -fa "v${version}" -m "$commit_msg" >> /tmp/aegispanel_release.log 2>&1
    echo -e "${OK}"

    # 6. Push zu GitHub
    echo -en "${ARROW} Push nach GitHub (Branch main + Tags)... "
    if git push "$auth_push_url" main --tags >> /tmp/aegispanel_release.log 2>&1; then
        echo -e "${OK}"
    else
        echo -e "${WARN} Normaler Push mit Fehlern. Versuche Push mit --force... "
        if git push "$auth_push_url" main --tags --force >> /tmp/aegispanel_release.log 2>&1; then
            echo -e "${OK}"
        else
            echo -e "${ERR} Push fehlgeschlagen! Log:"
            tail -10 /tmp/aegispanel_release.log
            return
        fi
    fi

    # 7. Release Asset komprimieren & GitHub Release anlegen
    local release_img="${IMAGES_DIR}/sdcard.img"
    local compressed_img="${IMAGES_DIR}/aegispanel-os-v${version}-rpi-zero2w.img.xz"
    local checksum_file="${IMAGES_DIR}/SHA256SUMS.txt"

    if [[ -f "$release_img" ]]; then
        echo -en "${ARROW} SD-Card-Image komprimieren (xz)... "
        if [[ ! -f "$compressed_img" ]]; then
            xz -k -f -T0 -9 -c "$release_img" > "$compressed_img"
        fi
        (cd "${IMAGES_DIR}" && sha256sum "$(basename "$compressed_img")" > "$checksum_file")
        echo -e "${OK} Fertig: $(du -h "$compressed_img" | cut -f1)"
    fi

    # 8. GitHub Release über REST API erstellen
    if [[ -n "$token" ]]; then
        local repo_path="moinmoin-64/aegispanel"

        echo -en "${ARROW} Erstelle GitHub Release v${version} via REST API... "
        local release_json
        release_json=$(curl -s -X POST \
            -H "Authorization: Bearer ${token}" \
            -H "Accept: application/vnd.github+json" \
            "https://api.github.com/repos/${repo_path}/releases" \
            -d @- <<EOF
{
  "tag_name": "v${version}",
  "name": "AegisPanel OS v${version}",
  "body": "${notes:-"AegisPanel OS Release v${version}"}",
  "draft": false,
  "prerelease": false
}
EOF
)
        local upload_url
        upload_url=$(echo "$release_json" | grep -o '"upload_url": *"[^"]*"' | head -1 | cut -d'"' -f4 | sed 's/{?name,label}//')

        if [[ -n "$upload_url" ]]; then
            echo -e "${OK}"
            if [[ -f "$compressed_img" ]]; then
                echo -en "${ARROW} Lade Release-Asset hoch ($(basename "$compressed_img"))... "
                curl -s -X POST \
                    -H "Authorization: Bearer ${token}" \
                    -H "Content-Type: application/x-xz" \
                    --data-binary @"$compressed_img" \
                    "${upload_url}?name=$(basename "$compressed_img")" > /dev/null
                echo -e "${OK}"
            fi
            if [[ -f "$checksum_file" ]]; then
                echo -en "${ARROW} Lade SHA256 Prüfsummen hoch... "
                curl -s -X POST \
                    -H "Authorization: Bearer ${token}" \
                    -H "Content-Type: text/plain" \
                    --data-binary @"$checksum_file" \
                    "${upload_url}?name=SHA256SUMS.txt" > /dev/null
                echo -e "${OK}"
            fi
            echo -e "${OK} ${BOLD}GitHub Release v${version} erfolgreich online veröffentlicht!${RESET}"
        else
            echo -e "${WARN} Release-Erstellung fehlgeschlagen. Antwort: $(echo "$release_json" | head -n 3)"
        fi
    fi
}

flash_wizard() {
    step_header "SD-Karte flashen"

    echo -e "${ARROW} Verfügbare Block-Geräte:"
    echo ""
    lsblk -o NAME,SIZE,TYPE,MOUNTPOINT | grep -v "loop" | head -20
    echo ""

    echo -e "${WARN} ${BOLD}Stelle sicher dass du das RICHTIGE Gerät wählst!${RESET}"
    local device
    device=$(ask "SD-Karten Gerät (z.B. sdb oder mmcblk0)" "")

    if [[ -z "$device" ]]; then
        echo -e "${ERR} Kein Gerät angegeben. Abbruch."
        return
    fi

    local dev_path="/dev/${device}"

    if [[ ! -b "${dev_path}" ]]; then
        echo -e "${ERR} ${dev_path} ist kein Block-Gerät!"
        return
    fi

    echo ""
    echo -e "${WARN} ${BOLD}${RED}LETZTE WARNUNG!${RESET}"
    echo -e "  Gerät:  ${BOLD}${dev_path}${RESET}"
    echo -e "  Image:  ${BOLD}${IMAGES_DIR}/sdcard.img${RESET}"
    echo -e "  Größe:  $(du -h "${IMAGES_DIR}/sdcard.img" | cut -f1)"
    echo ""

    if ! confirm "Jetzt flashen? Alle Daten auf ${dev_path} werden GELÖSCHT!" "n"; then
        echo -e "${WARN} Abgebrochen."
        return
    fi

    echo ""
    run_cmd_live "Flashen mit dd" \
        dd if="${IMAGES_DIR}/sdcard.img" of="${dev_path}" bs=4M status=progress conv=fsync

    run_cmd "Puffer leeren" sync

    echo -e "${OK} ${BOLD}SD-Karte erfolgreich geflasht!${RESET}"
    echo ""
    echo -e "${DIM}  Gerät sicher auswerfen: sudo eject ${dev_path}${RESET}"
}

summary() {
    local version="$1"
    step_header "Zusammenfassung"

    echo -e "${OK} ${BOLD}AegisPanel OS v${version} erfolgreich erstellt!${RESET}"
    echo ""
    echo -e "  ${DIM}Erzeugte Dateien:${RESET}"
    if [[ -f "${IMAGES_DIR}/sdcard.img" ]]; then
        echo -e "  ${OK} sdcard.img   $(du -h "${IMAGES_DIR}/sdcard.img" | cut -f1)"
    fi
    if [[ -f "${IMAGES_DIR}/rootfs.ext2" ]]; then
        echo -e "  ${OK} rootfs.ext2  $(du -h "${IMAGES_DIR}/rootfs.ext2" | cut -f1)"
    fi
    if [[ -f "${IMAGES_DIR}/Image" ]]; then
        echo -e "  ${OK} Image        $(du -h "${IMAGES_DIR}/Image" | cut -f1)"
    fi
    echo ""
    echo -e "  ${DIM}Flashen:${RESET}"
    echo -e "  ${CYAN}sudo dd if=${IMAGES_DIR}/sdcard.img of=/dev/sdX bs=4M status=progress conv=fsync${RESET}"
    echo ""
}

# ── Hauptprogramm ─────────────────────────────────────────
main() {
    > /tmp/aegispanel_release.log

    banner
    check_prereqs

    # Build-Typ wählen
    select_build_type

    # Versions-Wizard (nicht für reines Overlay-Update)
    SELECTED_NEW_VERSION=$(get_current_version)
    if [[ "$SELECTED_BUILD_TYPE" != "2" ]]; then
        version_wizard
    fi

    # Changelog-Eintrag (optional)
    step_header "Changelog"
    if confirm "Changelog-Eintrag hinzufügen?" "y"; then
        echo -e "${ARROW} ${BOLD}Was ist neu in v${SELECTED_NEW_VERSION}?${RESET} ${DIM}(Enter zum Überspringen)${RESET}"
        echo -en "  > "
        read -r CHANGELOG_MSG
        if [[ -n "$CHANGELOG_MSG" ]]; then
            local changelog_file="${PROJECT_DIR}/CHANGELOG.md"
            local date_str
            date_str=$(date '+%Y-%m-%d')
            local tmp_changelog="/tmp/aegis_changelog.tmp"
            
            {
                echo "# AegisPanel OS Changelog"
                echo ""
                echo "## v${SELECTED_NEW_VERSION} – ${date_str}"
                echo "- ${CHANGELOG_MSG}"
                echo ""
                if [[ -f "$changelog_file" ]]; then
                    tail -n +3 "$changelog_file" 2>/dev/null || true
                fi
            } > "$tmp_changelog"
            mv "$tmp_changelog" "$changelog_file"
            echo -e "${OK} Changelog aktualisiert"
        fi
    fi

    # Build ausführen
    case "$SELECTED_BUILD_TYPE" in
        1) build_rust_only "$SELECTED_NEW_VERSION" ;;
        2) build_overlay_only ;;
        3) build_full_incremental ;;
        4) build_clean ;;
        *) build_full_incremental ;;
    esac

    # SD-Card-Image erstellen
    step_header "SD-Card-Image"
    if confirm "SD-Card-Image jetzt erstellen?" "y"; then
        build_sdcard
    fi

    # GitHub Release & Repository Sync
    if confirm "Jetzt nach GitHub pushen und Release v${SELECTED_NEW_VERSION} online erstellen?" "y"; then
        github_release_wizard "$SELECTED_NEW_VERSION" "$CHANGELOG_MSG"
    fi

    # SD-Karte flashen
    step_header "Flashen"
    if confirm "SD-Karte jetzt flashen?" "n"; then
        flash_wizard
    fi

    summary "$SELECTED_NEW_VERSION"
}

main "$@"
