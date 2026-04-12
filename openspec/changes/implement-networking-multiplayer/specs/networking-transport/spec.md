## ADDED Requirements

### Requirement: WebRTC peer-to-peer transport via Matchbox
The system SHALL provide a WebRTC-based peer-to-peer transport layer using `matchbox_socket`. The transport SHALL work on native platforms (via tokio runtime) and in WASM (via browser WebRTC APIs). The transport SHALL establish data channels between peers after an initial signaling handshake. After connection, all game data SHALL flow peer-to-peer without routing through the signaling server.

#### Scenario: Native peer connects to signaling server
- **WHEN** a native client creates a `MatchboxTransport` with a signaling server URL and room ID
- **THEN** the transport SHALL connect to the signaling server, join the specified room, and begin the WebRTC handshake with other peers in the room

#### Scenario: WASM peer connects to signaling server
- **WHEN** a WASM client creates a `MatchboxTransport` with a signaling server URL and room ID
- **THEN** the transport SHALL use the browser's WebRTC API to connect to the signaling server, join the room, and establish peer connections

#### Scenario: Native-to-browser cross-platform connection
- **WHEN** a native client and a WASM client join the same room on the signaling server
- **THEN** a WebRTC data channel SHALL be established between them, enabling direct peer-to-peer game data exchange

### Requirement: Transport abstraction for alternative backends
The system SHALL define a `Transport` trait that abstracts the underlying network transport. The Matchbox WebRTC implementation SHALL implement this trait. The trait SHALL support `send`, `receive`, `connected_peers`, and `disconnect` operations, enabling future alternative backends (direct UDP, Steam networking) without changing the session layer.

#### Scenario: Send game data to a peer
- **WHEN** `transport.send(peer_id, data)` is called with a connected peer
- **THEN** the data SHALL be delivered to the specified peer via the underlying transport

#### Scenario: Receive game data from peers
- **WHEN** a peer sends data over the transport
- **THEN** `transport.receive()` SHALL return the data along with the sender's peer ID

#### Scenario: Query connected peers
- **WHEN** `transport.connected_peers()` is called
- **THEN** the system SHALL return the list of currently connected peer IDs

### Requirement: Connection lifecycle management
The transport SHALL handle connection establishment, maintenance, and teardown. The system SHALL detect peer disconnections and notify the session layer. The system SHALL support graceful disconnection (player leaves) and ungraceful disconnection (network failure, timeout).

#### Scenario: Peer disconnects gracefully
- **WHEN** a peer calls `transport.disconnect()`
- **THEN** all other peers SHALL be notified of the disconnection via a `PeerDisconnected` event

#### Scenario: Peer connection lost
- **WHEN** a peer's network connection is lost (no data received within the timeout period)
- **THEN** the transport SHALL emit a `PeerDisconnected` event for the lost peer after the timeout expires

#### Scenario: All peers disconnected
- **WHEN** all remote peers have disconnected
- **THEN** the transport SHALL report zero connected peers and the session layer SHALL handle the empty session (return to lobby or main menu)

### Requirement: Signaling server deployment
The system SHALL include deployment configuration for the Matchbox signaling server. The signaling server SHALL handle only WebRTC signaling handshakes (SDP offer/answer, ICE candidate exchange), not game traffic. The signaling server SHALL be deployable as a Docker container alongside the existing infrastructure.

#### Scenario: Signaling server accepts room connections
- **WHEN** a client connects to the signaling server with a room ID
- **THEN** the server SHALL add the client to the room and facilitate WebRTC handshake with other clients in the room

#### Scenario: Signaling server handles multiple rooms
- **WHEN** multiple rooms are active simultaneously
- **THEN** the signaling server SHALL isolate rooms so that clients only discover peers within their own room

### Requirement: GGRS-compatible socket implementation
The Matchbox transport SHALL implement the `ggrs::NonBlockingSocket` trait, enabling direct use as a GGRS transport backend. Input messages, sync data, and quality reports SHALL be transmitted over the WebRTC data channel with unreliable delivery (UDP-like semantics).

#### Scenario: GGRS sends input to peers
- **WHEN** GGRS calls `send_to` on the socket with an input message for a peer
- **THEN** the message SHALL be transmitted over the WebRTC data channel to that peer

#### Scenario: GGRS receives input from peers
- **WHEN** GGRS calls `receive_all_messages` on the socket
- **THEN** all pending messages from all peers SHALL be returned
