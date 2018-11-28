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
    for i in ${CLIENTLIST[@]}; do
        send $i "/leave"
    done
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
sleep 1
# First basic test... spam 500 /list and chat messages
roomCmdCount=500
until [[ $roomCmdCount -eq 0 ]];
do
    for i in ${CLIENTLIST[@]}; do
        IssueInRoomCommands $i
    done
    roomCmdCount=$((roomCmdCount-1))
done

LeaveRooms

# Impose arbitrary latency to outbound UDP
#
# https://serverfault.com/questions/336089/adding-latency-to-outbound-udp-packets-with-tc
# http://luxik.cdi.cz/~devik/qos/htb/manual/userg.htm
# https://wiki.linuxfoundation.org/networking/netem?s[]=netem

:'
# Handle "1:" is equivalent to <major>:<minor>... minor of qdisc is always 0.
#   This is the "root" qdisc with a handle of 1:
# Classes need to have the same major number as their parent.
#   The following command instantly creates classes 1:1, 1:2, and 1:3.
'
tc qdisc add dev eth0 root handle 1: prio

:'
# 1:3 refers to the third band of the PRIO queue.
# This described by qdisc ID of 30:
'
tc class add dev lo parent 1:3 classid 1:10 netem delay 200ms 20ms distribution normal    # Delayed 200
#tc class add dev eth0 parent 1:3 classid 1:20 netem loss 1%                                 # Dropped 1%
# 20% of packets sent immediately, rest delayed 10ms
#tc class add dev eth0 parent 1:3 classid 1:30 netem delay 10ms reorder 25% 50%              # out-of-order
#tc class add dev eth0 parent 1:3 classid 1:30 netem delay 10ms duplicate 10%                # duplicate 10%
#tc class add dev eth0 parent 1:3 classid 1:40 netem corrupt 1%                              # corrupt 1%

:'
# What packets belong in what qdisc?
#
# protocol ip   -- We are accepting IP traffic
# parent        -- Must already exist (1:0 as specified above)
# prio          -- Priority of this filter is set to 3. Lower the quicker.
# fw flowid     -- Bases the decision on how the firewall (iptables) marked the packet, to be processed by the 1:3 class
# handle        -- filter ID
'
tc filter add dev eth0 parent 1:0 protocol ip prio 3 handle 1 fw flowid 1:3

:'
# -o is for outbound; -i is for inbound
# Mangle (modify) packets before they leave (POSTROUTING) on eth0 udp traffic.
# Applied to class 1:3 <parent>:<child>
'
iptables -t mangle -A POSTROUTING -o eth0 -p udp -j CLASSIFY --set-class 1:3

# So all in all, IP tables marks that all UDP packets sent out are processed by '1:3' which enacts on the IP protocol,
# adding a delay of 200ms to each UDP packet.

# Goal will be to set up a few netem filters and use IP tables to control which one is in use.
#   1. Outgoing packet delay
#   2. Dropping outgoing packets
#   3. Re-ordering packets
#   4. Packet duplication
#   5. Packet corruption
#   6. Enable ALL in some fashion (dying connection)
