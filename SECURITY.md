# Security policy

Security fixes target the current 1.x release after publication. Before publication,
report problems against the development branch and include its commit hash.

Report suspected vulnerabilities privately to smmartbiz@gmail.com, with affected version,
minimal reproduction and impact. Please omit credentials and private crawl results and
allow investigation before public disclosure.

ahref requests the URLs you provide and internal links discovered from them. It is a local
CLI, not a sandbox for untrusted crawl jobs. If embedded in a service, the integrator must
enforce allowed destinations, DNS/IP restrictions and network isolation. Hostname scope
alone is not an SSRF defense. Never expose an unrestricted crawler as a public endpoint.

Graphs may contain private paths, query parameters, anchors and redirect destinations.
Review exports before sharing. The crawler does not execute scripts and does not send
results to a third-party service. Keep dependencies updated and honor site access rules.
