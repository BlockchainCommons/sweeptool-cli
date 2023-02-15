#
# Regular cron jobs for the sweeptool-cli package.
#
0 4	* * *	root	[ -x /usr/bin/sweeptool-cli_maintenance ] && /usr/bin/sweeptool-cli_maintenance
