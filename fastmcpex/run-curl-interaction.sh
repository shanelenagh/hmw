#!/bin/sh
set -x
echo ">>>> Sending initialization request"
init_output=$(. ./curl-init.sh 2>&1)
#echo ">>>> Init response: $init_output"
session_id_hdr=$(echo $init_output | grep -oP 'mcp-session-id: [\d\w]+')
echo ">>>> Initialized, and got Session ID Header: $session_id_hdr"
echo ">>>> Sending ACK of initialization"
. ./curl-nowinitialized.sh -H "$session_id_hdr"
echo ">>>> Listing tools"
. ./curl-listtools.sh -H "$session_id_hdr"
echo ">>>> Calling tool"
. ./curl-toolcall.sh -H "$session_id_hdr"