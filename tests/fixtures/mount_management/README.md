# Mount Management Fixtures

Fixture scaffolding for feature 005 native mount management.

Scenarios:
- `normal-nfs/`: successful mount-backed service using a network-backed mount
- `network-automount/`: explicit automount for a network-backed mount such as NFS
- `invalid-definition/`: invalid mount declaration and ownership/validation failures
- `busy-removal/`: previously managed mount that cannot be cleanly removed
