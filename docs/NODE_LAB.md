# Two-Pi development lab

## Scope and ownership

The first lab step is development coordination over a dedicated Ethernet cable.
GigPies owns the Stagebox/Brain control contract and integration. The
[component map](COMPONENTS.md) keeps PA DSP/measurement in SHR PA, effects in
SHR FX, and lighting in SHR Lux. See the original
[architecture](ARCHITECTURE.md) and [hardware handoff](NEXT_SESSION.md).

The development channel uses SSH, Git and small result files. It does not choose
the eventual live control or audio transport. Stagebox audio, monitors and local
protection must continue with the last valid state when Brain communication fails.
The Pi models' eventual Stagebox/Brain assignments remain open until profiling.

## Address plan

| Machine | Ethernet IPv4 | Initial responsibility |
|---|---|---|
| Pi 5 | `192.168.234.221/24` | Source integration and coordination repository |
| Pi 4 | `192.168.234.222/24` | Peer setup, inventory and assigned experiments |

Use no gateway or DNS on this dedicated link. Keep internet access on the existing
Wi-Fi connection. Persist configuration in the network manager's owning files;
when Netplan generates NetworkManager profiles, update Netplan too. An address
added with `ip addr` alone does not survive a reboot.

On 2026-10-03 the Pi 5's `eth0` negotiated 1000 Mb/s full duplex. Its static address,
saved Netplan generation and Wi-Fi default route were checked. The Pi 4 was not
yet configured, so connectivity, SSH, throughput and reconnect acceptance remain
pending. Pi 5 reports hardware timestamp capability; that alone does not establish
a shared clock or PTP accuracy. Machine configuration, backups and peer keys stay
outside published source.

## Authentication and source transfer

Password login on one node and key login on the other can coexist. Give each node
its own dedicated Ed25519 client key and exchange only public keys. Verify each
server's Ed25519 host fingerprint from its local console before pinning it. Keep
host checking enabled. Restrict the new authorized keys to the peer's Ethernet
source address and disable forwarding/PTY facilities with `restrict`.

Clone committed project history over SSH. The Pi 5 may have local commits absent
from GitHub, so its advertised commit manifest identifies the initial source set.
Never use directory mirroring to overwrite an active checkout. Never copy `.ssh`,
Codex credentials, build directories, recordings or private sessions as source.
Pin any experiment's exact commit and record working-tree changes explicitly.

The private coordination hub is a separate bare Git repository at
`user/node-lab/exchange.git` on Pi 5. Each node uses its own working clone. Its
README defines the message and handoff conventions. The parent project's ignore
rules keep that lab state out of GigPies publication. No GitHub push is required.

## Handoff loop

1. Pull the coordination repository with `--ff-only` before starting work.
2. Read new records for the current task. One node owns each module/file change;
   reserve a shared hardware or link test before starting it.
3. Post an immutable record under `nodes/rpi5/` or `nodes/rpi4/`: task ID, state,
   exact source revision, scope, command, evidence and next owner. Use a unique
   filename. States are `queued`, `running`, `ready-for-review`, `accepted`,
   `failed` or `cancelled`.
4. Commit only the named handoff files and push. A rejected concurrent push means
   fetch/rebase, inspect, then retry; never force-push. Keep local records when
   disconnected and retry after reconnection.
5. The receiver acknowledges a result in its own record. A Git push only proves
   delivery to the hub, not that the other session read or accepted the result.

Post source changes as a small patch/commit with its base revision. The receiver
reviews and applies it in its own checkout. Store concise measurements and hashes;
transfer larger explicitly requested artifacts separately with rsync and verify
their hashes. Do not mirror private media into the coordination repository.

During a coordinated experiment, each active session checks the repository before
work and while waiting for a peer. There is no session wake-up integration in this
setup. A session that is idle needs the operator to resume it. Timeouts retain work
and post a named blocker; a restart resumes from the last acknowledged task state.

## First experiments

Run one declared link experiment at a time and record both hosts' software,
interface, MTU, CPU load and clock status. Keep machine inventory in the private
coordination repository.

1. Verify bidirectional ping, pinned SSH and a small file's SHA-256 after transfer.
2. Record a ping RTT distribution and loss. RTT is not one-way latency.
3. Run bounded iperf3 TCP tests in both directions, then UDP at declared rates
   below link capacity; capture loss, jitter and CPU load. Bind the temporary
   server to the Ethernet address and stop it after the experiment.
4. Verify reconnect/restart recovery and repeated Git handoffs. Coordinate a
   cable pull or interface interruption so another experiment is not disrupted.
5. Implement a hardware-free versioned control prototype: node/session identity,
   command ID, sequence, expected state revision, bounded values, acknowledgment,
   heartbeat and state resynchronization. Test duplicate/stale commands, restarts,
   malformed/oversized messages, disconnects and manual ownership before audio.
6. Select audio transport, buffering and clock handling from measured needs.
   Source-frame counters and audio-clock drift need explicit treatment even if
   network time is synchronized. RTP/UDP and PTP remain candidates.

Normal software tests stay hardware-free. Link benchmarks and interruption trials
are explicit lab operations. Full-song studies, audio playback, MIDI and DMX are
separate from this initial network setup.

## References

- [NetworkManager IPv4 settings](https://networkmanager.dev/docs/api/latest/settings-ipv4.html)
  documents static addresses and `never-default` routing.
- [OpenSSH manuals](https://www.openssh.org/manual.html) document key authentication,
  host verification and authorized-key restrictions.
- [iperf3 invocation](https://software.es.net/iperf/invoking.html) documents bound
  servers, reverse tests, UDP load and JSON results; consult installed-version help.
