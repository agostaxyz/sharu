# Security policy

Please report vulnerabilities privately through GitHub: **Security → Report a vulnerability** on this repository
(private vulnerability reporting). Do not open public issues for security problems.

We will acknowledge reports within a few days and credit reporters who want credit.

Scope: the signalling server, the web app, the CI/CD configuration. Out of scope: third-party STUN/TURN servers
and your own hosting.

Design notes: session codes are bearer secrets (kept out of logs, `Referrer-Policy: no-referrer`); media is
end-to-end encrypted by WebRTC (DTLS-SRTP) between browsers; the server never sees video or audio in mesh mode.
