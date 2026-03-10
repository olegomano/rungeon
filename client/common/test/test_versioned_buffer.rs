use versioned_buffer::Commit;
use versioned_buffer::Trie;
#[test]
fn constructs_trie() {
    let mut t: Trie<i32> = Trie::new();

    let mut trie_write_0 = Commit::<i32>::new();
    trie_write_0.Write(0, 11);
    let root_0 = t.ApplyCommit(trie_write_0);

    let mut trie_write_1 = Commit::<i32>::new();
    trie_write_1.Write(0, 22);
    let root_1 = t.ApplyCommit(trie_write_1);

    println!("I AM RUNNING");
    t.Diff(root_0, root_1, &mut |key, old, new| {
        println!("{:?},{:?},{:?}", key, old, new);
    });
}
