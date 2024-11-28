#!/usr/bin/bash

pushd target
find . -name "*.Dockerfile" -exec rm {} \;
rm -f */publish/jira_worklog_tu*
rm -f */publish/rust-axum-backe*

mkdir jira-worklog
find */publish -type f | while read file; do
    filename="$(basename $file)"
    name="${filename%.*}"
        extension="${filename##*.}"

    IFS='-' read -r arch vendor os lib <<< "$(echo "$file" | grep -oP '^[^-]+-[^-]+-[^-]+-[^-]+(?=/publish)')"

    if [[ "$filename" == *.* ]]; then
        cp $file "jira-worklog/${name}-$1-${arch}-${vendor}-${os}-${lib}.${extension}"
    else
        cp $file "jira-worklog/${name}-$1-${arch}-${vendor}-${os}-${lib}"
    fi
done

popd
