# Security policy

Please report vulnerabilities privately through GitHub: **Security → Report a vulnerability** on this repository.
Do not open public issues for security problems. We will acknowledge reports within a few days and credit reporters who want credit.

Scope: the web app and the CI/CD configuration. Out of scope: third-party STUN servers and the public relays used to find peers.

Design notes: video and audio are encrypted by WebRTC (DTLS-SRTP) between browsers and never touch a relay. The session link is a bearer secret; relays can see that a room exists but not its contents. Anyone who has the link can join, so share it with people you trust.
