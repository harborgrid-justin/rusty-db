// Protocol Handlers Module
//
// Wire codec and protocol negotiation logic

use super::message_types::*;

// ============================================================================
// Wire Codec
// ============================================================================

pub struct WireCodec {
    compression: CompressionType,
}

impl WireCodec {
    pub fn new(compression: CompressionType) -> Self {
        Self { compression }
    }

    pub fn compression(&self) -> CompressionType {
        self.compression
    }

    pub fn set_compression(&mut self, compression: CompressionType) {
        self.compression = compression;
    }
}

#[derive(Debug, Clone)]
pub struct WireCodecMetrics {
    pub bytes_encoded: u64,
    pub bytes_decoded: u64,
}

impl Default for WireCodecMetrics {
    fn default() -> Self {
        Self {
            bytes_encoded: 0,
            bytes_decoded: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WireCodecStats {
    pub compression_ratio: f64,
}

// ============================================================================
// Protocol Negotiation
// ============================================================================

pub struct ProtocolNegotiator {
    capabilities: ProtocolCapabilities,
}

impl ProtocolNegotiator {
    pub fn new(capabilities: ProtocolCapabilities) -> Self {
        Self { capabilities }
    }

    pub fn capabilities(&self) -> &ProtocolCapabilities {
        &self.capabilities
    }

    /// Negotiate protocol with a client based on their capabilities
    pub fn negotiate(
        &self,
        client_capabilities: &ProtocolCapabilities,
    ) -> Option<NegotiatedProtocol> {
        // Find the highest compatible protocol version
        let version = ProtocolVersion::V1_0_0; // For now, always use v1.0.0

        // Find a mutually supported compression algorithm
        let compression = self
            .capabilities
            .compression
            .iter()
            .find(|c| client_capabilities.compression.contains(c))
            .copied()
            .unwrap_or(CompressionType::None);

        Some(NegotiatedProtocol {
            version,
            compression,
        })
    }
}
