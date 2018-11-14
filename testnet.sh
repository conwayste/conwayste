#!/bin/bash
#
# Conwayste Developers © 2018
#

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
    for i in ${CLIENTLIST[@]}; do
        tmux kill-session -t $i
    done
    tmux kill-session -t server
    tmux ls
}

function ConnectToServerDefaultTestRoom()
{
    send $1 "/connect Test$1"
    send $1 "/new TestRoom"
    send $1 "/join TestRoom"
}

function IssueInRoomCommands()
{
    rand=$((RANDOM % 3))
    case "$rand" in
        0)
            send $1 "Just one really really really really really really really really really long message."
            ;;
        1)
            send $1 "/list"
            ;;
        *)
            send $1 "Rust makes life better."
            send $1 "Don't you think?."
            ;;
    esac
}

function LeaveRooms()
{
    send client "/leave"
    send client2 "/leave"
}


CLIENTLIST=( client client2 )

tmuxkill
cargo build

tmux new-session -d -s server "export RUST_BACKTRACE=1; export RUST_LOG=server; ./target/debug/server; read"
for i in ${CLIENTLIST[@]}; do
    print "$i"
    tmux new-session -d -s $i "export RUST_BACKTRACE=1; export RUST_LOG=client; ./target/debug/client; read"
done

echo "Listing sessions..."
tmux ls

if detect_kde;
then
    for i in ${CLIENTLIST[@]}; do
        konsole --noclose -e tmux attach-session -t $i &
    done
    konsole --noclose -e tmux attach-session -t server &
else
# For others environments swap in your favorite terminal
    echo "Your terminal environment isn't specified. Open me up and add support."
    exit 0
fi

echo "Attached to tmux-sessions."

for i in ${CLIENTLIST[@]}; do
    ConnectToServerDefaultTestRoom $i
done

# First basic test... spam 500 /list and chat messages
roomCmdCount=500
until [[ $roomCmdCount -eq 0 ]];
do
    for i in ${CLIENTLIST[@]}; do
        IssueInRoomCommands $i
    done
    roomCmdCount=$((roomCmdCount-1))
done

: '
tc qdist add dev eth0 root handle 1: prio
tc qdisc add dev eth0 parent 1:3 handle 30: netem delay 200ms
tc filter add dev eth0 parent 1:0 protocol ip prio 3 handle 1 fw flowid 1:3

iptables -t mangle -A POSTROUTING -o eth0 -p udp -j CLASSIFY --set-class 1:3
'

LeaveRooms