// Compression Algorithms Module
//
// Provides various compression algorithms optimized for database workloads:
// - LZ4: Fast compression for general-purpose data
// - Zstandard-like: Balanced compression ratio and speed
// - Huffman: Entropy encoding for high compression
// - Dictionary: LZW-based compression
// - Column encodings: Specialized for columnar data (BitPacker, FOR, Delta, RLE)
// - Adaptive: Automatically selects best algorithm
// - Cascaded: Intelligently combines encodings for optimal compression

pub mod adaptive_compression;
pub mod column_encodings;
pub mod dictionary_compression;
pub mod lz4_compression;
pub mod zstd_compression;

// Re-export main types
pub use adaptive_compression::{AdaptiveCompressor, CascadedCompressor};
pub use column_encodings::{
    BitPacker, DeltaEncoder, EnhancedDictionaryEncoder, FOREncoder, RLEEncoder,
};
pub use dictionary_compression::DictionaryCompressor;
pub use lz4_compression::LZ4Compressor;
pub use zstd_compression::{HuffmanCompressor, ZstdCompressor};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::{CompressionLevel, Compressor};

    #[test]
    fn test_bit_packer() {
        let packer = BitPacker::new();
        let values = vec![1, 5, 3, 7, 2, 4, 6, 0];

        let bit_width = BitPacker::bits_needed(*values.iter().max().unwrap());
        assert_eq!(bit_width, 3);

        let packed = packer.pack_u32(&values, bit_width);
        let unpacked = packer.unpack_u32_fast(&packed, bit_width, values.len());

        assert_eq!(values, unpacked);
    }

    #[test]
    fn test_for_encoder() {
        let encoder = FOREncoder::new();
        let values = vec![1000, 1001, 1002, 1003, 1004, 1005];

        let encoded = encoder.encode(&values).unwrap();
        let decoded = encoder.decode(&encoded).unwrap();

        assert_eq!(values, decoded);

        let original_size = values.len() * 4;
        let ratio = original_size as f64 / encoded.len() as f64;
        assert!(
            ratio > 3.0,
            "FOR should achieve >3:1 compression on tight ranges"
        );
    }

    #[test]
    fn test_delta_encoder() {
        let encoder = DeltaEncoder::new();
        let values = vec![100, 101, 102, 103, 104, 105, 106, 107];

        let encoded = encoder.encode(&values).unwrap();
        let decoded = encoder.decode(&encoded).unwrap();

        assert_eq!(values, decoded);

        let original_size = values.len() * 4;
        let ratio = original_size as f64 / encoded.len() as f64;
        assert!(
            ratio > 5.0,
            "Delta should achieve >5:1 on monotonic sequences"
        );
    }

    #[test]
    fn test_rle_encoder() {
        let encoder = RLEEncoder::new();

        let data = vec![5, 5, 5, 5, 5, 7, 7, 7, 9];
        let encoded = encoder.encode(&data).unwrap();
        let decoded = encoder.decode(&encoded).unwrap();
        assert_eq!(data, decoded);

        let values = vec![1, 1, 1, 1, 2, 2, 2, 3, 3];
        let encoded = encoder.encode_u32(&values).unwrap();
        let decoded = encoder.decode_u32(&encoded).unwrap();
        assert_eq!(values, decoded);

        let repetitive = vec![42; 1000];
        let encoded = encoder.encode_u32(&repetitive).unwrap();
        let ratio = (repetitive.len() * 4) as f64 / encoded.len() as f64;
        assert!(
            ratio > 100.0,
            "RLE should achieve >100:1 on highly repetitive data"
        );
    }

    #[test]
    fn test_enhanced_dictionary_encoder() {
        let encoder = EnhancedDictionaryEncoder::new();

        let data = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![1, 2, 3],
        ];

        let encoded = encoder.encode(&data).unwrap();
        let decoded = encoder.decode(&encoded).unwrap();

        assert_eq!(data, decoded);

        let original_size: usize = data.iter().map(|v| v.len()).sum();
        let ratio = original_size as f64 / encoded.len() as f64;
        assert!(
            ratio > 1.5,
            "Dictionary should compress low-cardinality data"
        );
    }

    #[test]
    fn test_compression_ratios() {
        let compressor = CascadedCompressor::new();

        let sorted: Vec<u32> = (10000..11000).collect();
        let compressed = compressor.compress_u32(&sorted).unwrap();
        let ratio = (sorted.len() * 4) as f64 / compressed.len() as f64;
        println!("Sorted integers ratio: {:.2}:1", ratio);
        assert!(ratio > 8.0, "Should achieve >8:1 on sorted integers");

        let timestamps: Vec<u32> = (0..1000).map(|i| 1609459200 + i).collect();
        let compressed = compressor.compress_u32(&timestamps).unwrap();
        let ratio = (timestamps.len() * 4) as f64 / compressed.len() as f64;
        println!("Timestamps ratio: {:.2}:1", ratio);
        assert!(ratio > 10.0, "Should achieve >10:1 on timestamps");

        let repetitive = vec![42; 10000];
        let compressed = compressor.compress_u32(&repetitive).unwrap();
        let ratio = (repetitive.len() * 4) as f64 / compressed.len() as f64;
        println!("Repetitive data ratio: {:.2}:1", ratio);
        assert!(ratio > 20.0, "Should achieve >20:1 on repetitive data");
    }
}
