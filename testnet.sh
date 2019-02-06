#!/bin/bash
#
# Conwayste Developers © 2018
#

CLIENTLIST=( client )

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
    for i in ${CLIENTLIST[@]}; do
        send $i "/leave"
    done
}

function Disconnect()
{
    for i in ${CLIENTLIST[@]}; do
        send $i "/disconnect"
    done
}

function PrintStats()
{
    echo "Printing Statistics to Clients windows."
    for i in ${CLIENTLIST[@]}; do
        send $i "/stats"
    done
}

function EnableTrafficControl()
{
    # Impose arbitrary latency to outbound UDP
    #
    # https://serverfault.com/questions/336089/adding-latency-to-outbound-udp-packets-with-tc
    # http://luxik.cdi.cz/~devik/qos/htb/manual/userg.htm
    # https://wiki.linuxfoundation.org/networking/netem?s[]=netem
    # https://gist.github.com/keturn/541339
    # http://home.ifi.uio.no/paalh/students/AndersMoe.pdf

    #sudo tc qdisc add dev eth0 root netem delay 200ms 40ms 25% loss 15.3% 25% duplicate 1% corrupt 0.1% reorder 5% 50%

    sudo tc qdisc add dev lo root handle 1: prio
    sudo tc qdisc add dev lo parent 1:3 handle 30: netem delay 200ms 40ms 25% loss 15.3% 25% duplicate 1% corrupt 0.1% reorder 5% 50%
    sudo tc filter add dev lo parent 1:0 protocol ip u32 match ip dport 12345 0xffff flowid 1:3
    sudo tc filter add dev lo parent 1:0 protocol ip u32 match ip sport 12345 0xffff flowid 1:3
    sudo tc qdisc show
}

function DisableTrafficControl()
{
    sudo tc qdisc del dev lo root
}

function main()
{
    DisableTrafficControl # Clean up from a previous session if needed
    tmuxkill
    cargo build

    tmux new-session -d -s server "export RUST_BACKTRACE=1; export RUST_LOG=server; ./target/debug/server; read"
    for i in ${CLIENTLIST[@]}; do
        printf "Starting client $i"
        tmux new-session -d -s $i "export RUST_BACKTRACE=1; export RUST_LOG=client; ./target/debug/client; read"
    done

    echo "Listing sessions..."
    tmux ls

    echo "Launching Terminal Tmux Windows..."
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

    # Delay to start tmux logging
    sleep 10

    echo "Enabling Traffic Control..."
    EnableTrafficControl

    # First basic test... spam 500 /list and chat messages
    roomCmdCount=1

    echo "Basic test: Spam $roomCmdCount /list and chat messages"

    until [[ $roomCmdCount -eq 0 ]];
    do
        for i in ${CLIENTLIST[@]}; do
            IssueInRoomCommands $i
        done
        roomCmdCount=$((roomCmdCount-1))
    done
    echo "....Done."

    echo "Wait to disable Traffic Control..."
    sleep 10
    echo "Disabling Traffic Control..."
    DisableTrafficControl

    PrintStats
    LeaveRooms
    #Disconnect
}

main