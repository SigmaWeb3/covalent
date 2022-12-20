use ethereum_types::H256;

pub type Hash = H256;

#[derive(Default)]
pub struct Hasher;

impl Hasher {
    pub fn digest_<T: AsRef<[u8]>>(input: T) -> Hash {
        H256(*blake3::hash(input.as_ref()).as_bytes())
    }
}

impl cita_trie::Hasher for Hasher {
    const LENGTH: usize = 32;

    fn digest(&self, data: &[u8]) -> Vec<u8> {
        H256(*blake3::hash(data).as_bytes()).0.to_vec()
    }
}
