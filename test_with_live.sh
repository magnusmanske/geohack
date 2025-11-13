#!/bin/bash
while IFS= read -r line
do
  # each line needs to start with "geohack.php?"

  url_local="http://localhost:8000/$line"
  #echo "$url_local"

  url_live="https://geohack.toolforge.org/$line"
  #echo "$url_live"

  curl -sg "$url_live" > live.html
  curl -sg "$url_local" > local.html

  live_bytes=$(wc -c < live.html)
  local_bytes=$(wc -c < local.html)

  echo "$line : $live_bytes / $local_bytes"
  diff -b live.html local.html
done < test_params.txt
