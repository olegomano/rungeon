use versioned_buffer::Commit;
use versioned_buffer::Trie;
#[test]
fn constructs_trie() {
    let mut t: Trie<i32> = Trie::new();

    let write_0 = t.WriteKey(0, 0);
    let write_1 = t.WriteKey(0, 1);
    let write_2 = t.WriteKey(1, 1);

    t.Diff(write_0, write_2, &mut |key, left, right| {
        println!("{}, {:?} -> {:?}", key, left, right);
    });
}
