use crate::error::SrpError;
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use srp::{
    client::{SrpClient, SrpClientVerifier},
    groups::G_2048,
    server::{SrpServer, SrpServerVerifier},
};
use zeroize::{Zeroize, Zeroizing};

pub const DEFAULT_USERNAME: &[u8] = b"chat";

/// Client-side SRP-6a session state machine.
pub struct SrpClientSession {
    pub username: Vec<u8>,
    password: Zeroizing<Vec<u8>>,
    a: Zeroizing<[u8; 64]>,
    pub a_pub: Vec<u8>,
}

pub struct SrpClientAwaitingServer {
    verifier: SrpClientVerifier<Sha256>,
    pub client_proof: Vec<u8>,
}

impl SrpClientSession {
    /// Starts SRP authentication: generates secret ephemeral `a` and computes public ephemeral `A`.
    pub fn new(username: &str, password: &str) -> Self {
        let client = SrpClient::<Sha256>::new(&G_2048);
        let mut a = Zeroizing::new([0u8; 64]);
        OsRng.fill_bytes(a.as_mut());
        let a_pub = client.compute_public_ephemeral(a.as_ref());

        Self {
            username: username.as_bytes().to_vec(),
            password: Zeroizing::new(password.as_bytes().to_vec()),
            a,
            a_pub,
        }
    }

    /// Processes server challenge (`salt` and server public ephemeral `B`).
    /// Computes client proof `M` to send to server.
    pub fn process_challenge(
        mut self,
        salt: &[u8],
        b_pub: &[u8],
    ) -> Result<SrpClientAwaitingServer, SrpError> {
        let client = SrpClient::<Sha256>::new(&G_2048);
        let verifier = client
            .process_reply(
                self.a.as_ref(),
                DEFAULT_USERNAME,
                self.password.as_ref(),
                salt,
                b_pub,
            )
            .map_err(|_| SrpError::InvalidParameter("invalid server ephemeral B".to_string()))?;

        // Zeroize ephemeral secrets immediately after computing verifier
        self.a.zeroize();
        self.password.zeroize();

        let client_proof = verifier.proof().to_vec();
        Ok(SrpClientAwaitingServer {
            verifier,
            client_proof,
        })
    }
}

impl SrpClientAwaitingServer {
    /// Verifies server proof `H_AMK`.
    /// Returns the mutually authenticated SRP session key on success.
    pub fn verify_server(self, server_proof: &[u8]) -> Result<Zeroizing<Vec<u8>>, SrpError> {
        self.verifier
            .verify_server(server_proof)
            .map_err(|_| SrpError::ServerVerificationFailed)?;

        Ok(Zeroizing::new(self.verifier.key().to_vec()))
    }
}

/// Server-side SRP-6a verifier and session manager.
#[derive(Clone)]
pub struct ChatSrpServer {
    salt: Vec<u8>,
    verifier: Vec<u8>,
}

pub struct ServerHandshake {
    verifier: SrpServerVerifier<Sha256>,
    pub b_pub: Vec<u8>,
}

impl ChatSrpServer {
    /// Creates a server SRP configuration from the room password and salt.
    pub fn new(password: &str, salt: &[u8]) -> Self {
        let client = SrpClient::<Sha256>::new(&G_2048);
        let pwd_verifier = client.compute_verifier(DEFAULT_USERNAME, password.as_bytes(), salt);

        Self {
            salt: salt.to_vec(),
            verifier: pwd_verifier,
        }
    }

    pub fn salt(&self) -> &[u8] {
        &self.salt
    }

    /// Handles client public ephemeral `A`. Generates ephemeral `b`, computes `B`, and creates handshake state.
    pub fn init_handshake(&self, client_public_a: &[u8]) -> Result<ServerHandshake, SrpError> {
        let server = SrpServer::<Sha256>::new(&G_2048);
        let mut b = Zeroizing::new([0u8; 64]);
        OsRng.fill_bytes(b.as_mut());

        let b_pub = server.compute_public_ephemeral(b.as_ref(), &self.verifier);
        let verifier = server
            .process_reply(b.as_ref(), &self.verifier, client_public_a)
            .map_err(|_| SrpError::InvalidParameter("invalid client ephemeral A".to_string()))?;

        Ok(ServerHandshake { verifier, b_pub })
    }
}

impl ServerHandshake {
    /// Verifies client proof `M`. If valid, returns `(server_proof, session_key)`.
    pub fn verify_client_proof(
        self,
        client_proof: &[u8],
    ) -> Result<(Vec<u8>, Zeroizing<Vec<u8>>), SrpError> {
        self.verifier
            .verify_client(client_proof)
            .map_err(|_| SrpError::ClientVerificationFailed)?;

        let server_proof = self.verifier.proof().to_vec();
        let session_key = Zeroizing::new(self.verifier.key().to_vec());
        Ok((server_proof, session_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srp_handshake_success() {
        let salt = [42u8; 16];
        let password = "correct-horse-battery-staple";

        let server = ChatSrpServer::new(password, &salt);
        let client_session = SrpClientSession::new("alice", password);

        // 1. Client sends A to server; server creates handshake and sends B
        let server_handshake = server
            .init_handshake(&client_session.a_pub)
            .expect("server init handshake");

        // 2. Client processes salt and B, produces proof M
        let client_awaiting = client_session
            .process_challenge(&salt, &server_handshake.b_pub)
            .expect("client process challenge");

        // 3. Server verifies client proof M, produces server proof H_AMK and session key
        let (server_proof, server_session_key) = server_handshake
            .verify_client_proof(&client_awaiting.client_proof)
            .expect("server verifies client proof");

        // 4. Client verifies server proof H_AMK and gets session key
        let client_session_key = client_awaiting
            .verify_server(&server_proof)
            .expect("client verifies server proof");

        // 5. Both session keys must match!
        assert_eq!(*client_session_key, *server_session_key);
        assert!(!client_session_key.is_empty());
    }

    #[test]
    fn test_wrong_password_rejected() {
        let salt = [42u8; 16];
        let server_password = "server-password";
        let client_password = "wrong-password";

        let server = ChatSrpServer::new(server_password, &salt);
        let client_session = SrpClientSession::new("alice", client_password);

        let server_handshake = server
            .init_handshake(&client_session.a_pub)
            .expect("server init handshake");

        let client_awaiting = client_session
            .process_challenge(&salt, &server_handshake.b_pub)
            .expect("client process challenge");

        // Server MUST reject wrong password proof
        let verify_res = server_handshake.verify_client_proof(&client_awaiting.client_proof);
        assert!(matches!(
            verify_res,
            Err(SrpError::ClientVerificationFailed)
        ));
    }

    #[test]
    fn test_invalid_client_a_rejected() {
        let salt = [42u8; 16];
        let server = ChatSrpServer::new("password", &salt);

        // A = 0 or empty is invalid in SRP
        let result = server.init_handshake(&[]);
        assert!(result.is_err());
    }
}
