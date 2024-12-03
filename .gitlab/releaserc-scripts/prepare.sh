#!/usr/bin/bash

pushd target
find . -name "*.Dockerfile" -exec rm {} \;

mkdir timesheet
find */publish -type f | while read file; do
    filename="$(basename $file)"
    name="${filename%.*}"
        extension="${filename##*.}"

    IFS='-' read -r arch vendor os lib <<< "$(echo "$file" | grep -oP '^[^-]+-[^-]+-[^-]+-[^-]+(?=/publish)')"

    if [[ "$filename" == *.* ]]; then
        cp $file "timesheet/${name}-$1-${arch}-${vendor}-${os}-${lib}.${extension}"
    else
        cp $file "timesheet/${name}-$1-${arch}-${vendor}-${os}-${lib}"
    fi
done

popd
