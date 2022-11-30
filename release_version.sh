VERSION=$1
CRATES=(
    "tandem"
    "tandem_garble_interop"
    "tandem_http_client"
    "tandem_http_server"
)
for TOML in "${CRATES[@]}"; do
    SED_COMMAND="3s/^version = "[^,]*"/version = \"$VERSION\"/"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' -e "$SED_COMMAND" "${TOML}/Cargo.toml"
    else
        sed -i "$SED_COMMAND" "${TOML}/Cargo.toml"
    fi
    for CRATE in "${CRATES[@]}"; do
        SED_COMMAND="s/$CRATE = { version = "[^,]*"/$CRATE = { version = \"$1\"/"
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' -e "$SED_COMMAND" "$TOML/Cargo.toml"
        else
            sed -i "$SED_COMMAND" "$TOML/Cargo.toml"
        fi
    done
done

cargo build --all-features && \
    git commit -am "bump to v$VERSION" && \
    git tag -a "v$VERSION" -m "v$VERSION" && \
    git push --follow-tags
