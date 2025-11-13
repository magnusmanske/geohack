#!/bin/bash
while IFS= read -r line
do
  # each line needs to start with "geohack.php?"

  url_local="http://localhost:8000/$line"
  url_live="https://geohack.toolforge.org/$line"

  curl -sg "$url_live" > live.html
  curl -sg "$url_local" > local.html

  diff=$(diff -b live.html local.html)
  if [ -z "$diff" ]; then
    echo "$line : OK"
  else
    live_bytes=$(wc -c < live.html)
    local_bytes=$(wc -c < local.html)
    echo
    echo "$line : $live_bytes / $local_bytes bytes"
    echo "$diff"
    echo
  fi

done < test_params.txt
