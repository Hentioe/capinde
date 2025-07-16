#!/usr/bin/env sh

echo "Clearing xbps cache..."
rm -rf /var/cache/xbps/*

echo "Clearing xbps repository..."
find /var/db/xbps/ -type d -name "https___repo-*" -exec rm -rf {} +
