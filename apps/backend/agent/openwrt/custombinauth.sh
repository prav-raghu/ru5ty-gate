#!/bin/sh

method="$1"
mac="$2"
admin_url="http://127.0.0.1:2081/binauth"

case "$mac" in
	[0-9a-fA-F][0-9a-fA-F]:[0-9a-fA-F][0-9a-fA-F]:[0-9a-fA-F][0-9a-fA-F]:[0-9a-fA-F][0-9a-fA-F]:[0-9a-fA-F][0-9a-fA-F]:[0-9a-fA-F][0-9a-fA-F]) ;;
	*) exit 0 ;;
esac

case "$method" in
	client_deauth|idle_deauth|timeout_deauth|downquota_deauth|upquota_deauth|ndsctl_deauth|shutdown_deauth)
		body="{\"mac\":\"$mac\",\"method\":\"$method\"}"
		uclient-fetch -q -T 3 -O /dev/null \
			--header="Content-Type: application/json" \
			--post-data="$body" "$admin_url" >/dev/null 2>&1 &
		;;
esac

exit 0
