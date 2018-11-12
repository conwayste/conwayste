#!/bin/bash

# $1: client or server
# $2: command to send
function send()
{
    tmux send-keys -t $1 -l "$2"
    tmux send-keys -t $1 Enter
    sleep 0.0001
}

# Detection lifted from
# https://unix.stackexchange.com/questions/116539/how-to-detect-the-desktop-environment-in-a-bash-script
function detect_kde()
{
    ps -e | grep -E '^.* kded4$' > /dev/null
    if [ $? -ne 0 ];
    then
        return 0
    else
#        VERSION=`kded4 --version | grep -m 1 'KDE' | awk -F ':' '{print $2}' | awk '{print $1}'`
#        DESKTOP="KDE"
        return 1
    fi
}

function tmuxkill()
{
    tmux kill-session -t client
    tmux kill-session -t server
    tmux ls
}

tmuxkill
cargo build

tmux new-session -d -s server "export RUST_BACKTRACE=1; ./target/debug/server; read"
tmux new-session -d -s client "export RUST_BACKTRACE=1; ./target/debug/client; read"

echo "Listing sessions..."
tmux ls

if detect_kde;
then
    konsole --noclose -e tmux attach-session -t client &
    konsole --noclose -e tmux attach-session -t server &
else
# For others environments swap in your favorite terminal
    echo "Your terminal environment isn't specified. Open me up and add support"
    exit 0
fi

echo "Attached to tmux-sessions..."

send client "/connect TestUser1"
: '
send client "/new TestRoom"
send client "/join TestRoom"
send client "/list"
send client "/leave"
send client "/disconnect"
'