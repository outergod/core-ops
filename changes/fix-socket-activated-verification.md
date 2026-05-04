---
change_id: fix-socket-activated-verification
release_intent: patch
summary: Verifier no longer fails on socket-activated services that are correctly Inactive (their listening sockets are Active and will trigger the service on demand); ConfigFile and SocketDropIn workloads no longer contribute alias entries that misattribute real service failures to their config files
scope: verifier
release_preparation: false
---
