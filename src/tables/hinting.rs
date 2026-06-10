/// Hinting-related tables are stored as raw bytes and written back directly.
/// - `cvt `: Control Value Table (array of FWORD values)
/// - `prep`: CVT Program (bytecode)
/// - `fpgm`: Font Program (bytecode)
pub type HintingTable = Vec<u8>;
