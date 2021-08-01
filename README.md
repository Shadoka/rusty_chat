# Rusty Chat

Rusty Chat aims to be a (relatively) decentralized chatting tool to be used via command line.
It has three parts:

 1. server[executable]: Used as discovery server. New clients connect to a server of their choice and can get into contact with other clients.
 2. client[executable]: Peer-to-peer client and server logic. Additionally responsible for making first contact with a server and managing the UI.
 3. common[library]: Has all the structures used by both server and client.

# How it works

Clients find each other via discovery server. When two parties want to chat with each other the discovery server selects a client at random to be the new server for this new bidirectional chat. The required information is sent to both clients which then proceed to terminate the connection with the discovery server. The new dedicated server spins up his server and waits for the other client to connect. After a successful connection has been established both clients can start chatting.

# Known issues

A loooooooooot! Here is an assorted collection:

 - Currently the first client to connect to the discovery server has to go in a temporary 'wait mode'. That needs to go.
 - Discovery server is hardcoded to be local.
 - Clients exit after quitting on a chat. Should be guided back to discovery server.
 - The library used to print (colored) text to the command line should be cross platform, but I didn't test it myself.
 - There were problems using cygwin shell
 - lots of random debug output

# Missing features

 - group chat
 - encrypted communication
 - hole punching
 - I _could_ think about file transfer atleast in bidirectional chat

# How to run it

 - clone this repo
 - cd into server component and execute cargo run
 - cd into client component and execute cargo run