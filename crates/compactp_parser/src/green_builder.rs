//! Green-tree builder: turns the sink's flat start/token/finish stream
//! into a rowan [`GreenNode`], deduplicating repeated subtrees.
//!
//! # Why not `rowan::GreenNodeBuilder`
//!
//! rowan ships a builder that does the same job, and this crate used it
//! until [issue #22](https://github.com/devrelaicom/compactp/issues/22).
//! Its interner stores each cached node in a hash table whose *rehash*
//! callback is `node_hash`, which recomputes a node's hash by walking
//! its entire subtree recursively. Every time that table grows it
//! rehashes every live entry, so one growth costs the sum of all cached
//! subtree sizes.
//!
//! For a balanced tree that sum is `O(n log n)` and nobody notices. For
//! a *left-deep spine* — which is exactly what a left-associative
//! operator chain (`x+x+…+x`) or a postfix chain (`1()()…`, `x.a.a…`)
//! produces — subtree sizes run `1, 2, 3, …, n`, so the sum is
//! `Θ(n²)`. 20,000 terms spent 1.6 s of a release build inside
//! `node_hash`, ~90% of total parse time, and the cost quadrupled each
//! time the input doubled.
//!
//! Replacing the builder is the only available fix. `NodeCache` has no
//! `clear`, no capacity bound and no way to switch interning off;
//! `with_cache` only lets callers *share* a cache, which makes the
//! growth worse; the builder owns the child stack, so it cannot be
//! swapped mid-tree; and `[patch.crates.io]` does not survive
//! `cargo publish`, so a crate that ships on crates.io cannot pin a
//! patched rowan. Forking rowan is strictly more surface area than this
//! module.
//!
//! The fix is to give every interned element a dense integer identity
//! and key the table on `(kind, child identities)` instead of on the
//! subtree itself. The key is fixed-size and `Copy`, so rehashing an
//! entry is `O(1)` and no step of tree construction ever walks a
//! subtree it has already finished.
//!
//! This is a transliteration of rowan's relation, not a new one: rowan
//! keys its cache on `(kind, child *pointer* identity)` — see
//! `element_id` in its `node_cache.rs` — under the same "interned only
//! if all children are" invariant. Dense integers stand in for the
//! pointers, and neither can be recycled, for the same reason: the map
//! holds a strong reference to everything it has interned and never
//! evicts.
//!
//! Two identities are equal only if the elements they name are
//! structurally identical, by induction: a token's identity is
//! determined by its `(kind, text)`, and a node's by its kind and the
//! identities of its children. Reuse is therefore exact, and because
//! identities are handed out in a deterministic order the tree built
//! from a given event stream is always the same one.
//!
//! # Cost of the change
//!
//! Interning is preserved, with the same equality relation, so the
//! *retained tree* shares exactly the subtrees it shared before. The
//! *transient* tables are wider: a `NodeKey` plus its value is ~48 bytes
//! against rowan's 8, and a token entry ~40 against 8. Peak RSS rose
//! from 4.22 to 4.94 MiB on a 74 KB input and from 64.75 to 70.03 MiB
//! on a 1.48 MB one. That footprint is also why `parse small` and
//! `parse medium` are a few percent slower: a dedup-heavy steady state
//! touches more cache per lookup. Whole-corpus throughput still
//! improves, 88.0 to 91.4 MiB/s.
//!
//! # Upstream
//!
//! The rowan defect is real and small: `NodeCache` already stores each
//! child's hash beside it in `children: Vec<(u64, GreenElement)>`, so
//! the rehash closure could be `|(h, _)| *h` instead of `node_hash`.
//! Not filed upstream as of this commit. If it is fixed there, this
//! module can go away.

use rowan::{GreenNode, GreenToken, NodeOrToken, SyntaxKind};
use std::collections::hash_map::Entry;
use std::hash::{BuildHasherDefault, Hash, Hasher};

type GreenElement = NodeOrToken<GreenNode, GreenToken>;
type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<FxHasher>>;

/// Multiplier from the `FxHash` construction used by rustc and, before
/// this module existed, by the rowan builder these tables replace.
const FX_SEED: u64 = 0x517c_c1b7_2722_0a95;

/// Hasher for the interner's tables.
///
/// The standard library's default is SipHash-1-3 under a per-process
/// random key, and both halves of that are wrong here. Every token in
/// the input is looked up in these tables, so the hash sits on the hot
/// path — swapping SipHash in cost ~55% of parse throughput on
/// whitespace-heavy input when this module was first written. And the
/// random key makes internal behaviour vary between runs for no gain:
/// the tables are never iterated, and the hash only decides *bucket
/// placement*, never whether two elements are equal, so it cannot move
/// a single byte of the tree, the AST or the diagnostics.
///
/// A weak hash does mean a crafted input could force bucket collisions,
/// which cost extra probing. That is the same exposure rowan's builder
/// carried, and a collision here degrades to a missed deduplication,
/// never to a wrong tree.
///
/// Bytes are consumed little-endian so the hash is identical on every
/// target.
#[derive(Default)]
struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(FX_SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            self.add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
        }
        // Assembled with shifts rather than by copying into a buffer:
        // most tokens are shorter than eight bytes, so this tail runs
        // for nearly every token in the input, and `copy_from_slice`
        // leaves a call to `memmove` behind on that path.
        let tail = chunks.remainder();
        if !tail.is_empty() {
            let mut word = 0u64;
            for (i, &byte) in tail.iter().enumerate() {
                word |= u64::from(byte) << (i * 8);
            }
            self.add(word);
        }
    }

    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.add(u64::from(n));
    }

    #[inline]
    fn write_u16(&mut self, n: u16) {
        self.add(u64::from(n));
    }

    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.add(u64::from(n));
    }

    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }

    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// Identity of an interned green element, assigned in creation order.
///
/// `NOT_INTERNED` marks an element that was built without being
/// interned, and so cannot be named by identity.
///
/// 64 bits rather than 32 so the counter cannot be exhausted: it
/// advances once per *distinct* element, and every element needs at
/// least one byte of source, so wrapping would take an exabyte-scale
/// input. That matters because a wrapped identity would name two
/// different subtrees, and the interner would then share one where the
/// other belongs.
type Id = u64;

const NOT_INTERNED: Id = 0;

/// Widest node that is worth interning, in children.
///
/// Matches the threshold rowan uses. Dedup pays off for the small,
/// endlessly repeated shapes (`NAME_EXPR(IDENT)`, `FIELD_TYPE(Field)`);
/// wide nodes are nearly always unique, so interning them would cost a
/// lookup per node to save nothing.
const MAX_INTERNED_CHILDREN: usize = 3;

/// Share of an input's raw token count to reserve in each table.
///
/// Both tables start empty on every parse, so without a reservation
/// every file pays the same handful of grow-and-rehash steps on its way
/// up — measurably, ~5% of parse time. The caller knows the raw token
/// count, which is a hard upper bound on the distinct tokens, and
/// reserving some fraction of it skips those steps.
///
/// A quarter is where the measurements land. Across `tests/corpus`,
/// distinct tokens are 17.7% of raw tokens overall and 43.8% for the
/// median file — small files reuse little, large files reuse a lot.
/// Sweeping the divisor over the corpus, a quarter parses ~7% faster
/// than no reservation and ~3% faster than reserving the full token
/// count, which over-allocates enough that zeroing the table costs more
/// than the rehashes it avoids. Peak RSS moves by ~0.15 MiB on a 434 KB
/// input either way.
///
/// It only affects speed: an under- or over-estimate changes nothing
/// about the tree.
const RESERVED_TOKEN_SHARE: usize = 4;

/// Interning key for a node: its kind plus the identities of its
/// children. Fixed-size and `Copy`, so hashing it is `O(1)` — that is
/// the whole point (see the module docs).
#[derive(PartialEq, Eq, Clone, Copy)]
struct NodeKey {
    kind: SyntaxKind,
    len: u8,
    children: [Id; MAX_INTERNED_CHILDREN],
}

/// Hashes only the occupied child slots. Derived `Hash` would hash the
/// padding too — three words for a one-child node — and a length prefix
/// on top. Unused slots are always `NOT_INTERNED`, so equal keys still
/// hash equal and this stays consistent with the derived `PartialEq`.
impl Hash for NodeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u16(self.kind.0);
        state.write_u8(self.len);
        for id in self.children.iter().take(usize::from(self.len)) {
            state.write_u64(*id);
        }
    }
}

/// Builds a green tree from a flat start/token/finish stream.
pub(crate) struct GreenBuilder<'src> {
    /// Open nodes, as `(kind, index of first child in `children`)`.
    parents: Vec<(SyntaxKind, usize)>,
    /// Finished elements not yet claimed by a parent, each with its
    /// identity.
    children: Vec<(Id, GreenElement)>,
    nodes: HashMap<NodeKey, (Id, GreenNode)>,
    tokens: HashMap<(SyntaxKind, &'src str), (Id, GreenToken)>,
    next_id: Id,
}

impl<'src> GreenBuilder<'src> {
    /// `raw_tokens` is the number of tokens the lexer produced, used
    /// only to size the interner tables — see [`RESERVED_TOKEN_SHARE`].
    pub(crate) fn new(raw_tokens: usize) -> Self {
        let reserve = raw_tokens / RESERVED_TOKEN_SHARE;
        Self {
            parents: Vec::new(),
            children: Vec::new(),
            nodes: HashMap::with_capacity_and_hasher(reserve, Default::default()),
            tokens: HashMap::with_capacity_and_hasher(reserve, Default::default()),
            next_id: NOT_INTERNED,
        }
    }

    /// Append a token to the current node.
    ///
    /// The cache starts empty on every parse, so most lookups here are
    /// misses. Both this and [`GreenBuilder::node`] therefore go through
    /// the `entry` API, which finds the vacant slot and fills it in one
    /// probe — a `get`-then-`insert` pair costs two, on the path taken
    /// most often.
    #[inline]
    pub(crate) fn token(&mut self, kind: SyntaxKind, text: &'src str) {
        let Self {
            tokens,
            next_id,
            children,
            ..
        } = self;
        let (id, token) = tokens.entry((kind, text)).or_insert_with(|| {
            *next_id += 1;
            (*next_id, GreenToken::new(kind, text))
        });
        children.push((*id, token.clone().into()));
    }

    /// Open a node. Every element added until the matching
    /// [`GreenBuilder::finish_node`] becomes one of its children.
    #[inline]
    pub(crate) fn start_node(&mut self, kind: SyntaxKind) {
        self.parents.push((kind, self.children.len()));
    }

    /// Close the innermost open node.
    ///
    /// A close with nothing open is ignored rather than panicked on: the
    /// sink's event stream is balanced by construction, but the parser
    /// must never abort on user input.
    #[inline]
    pub(crate) fn finish_node(&mut self) {
        let Some((kind, first_child)) = self.parents.pop() else {
            return;
        };
        let entry = self.node(kind, first_child);
        self.children.push((entry.0, entry.1.into()));
    }

    /// Whether the stream so far closes to exactly one `root` node.
    ///
    /// `rowan::GreenNodeBuilder::finish` asserted this, and that
    /// assertion was doing real work: it passed on every input this
    /// project has ever parsed, which is stronger evidence that the
    /// grammar keeps its events balanced than any argument from
    /// inspection. [`GreenBuilder::finish`] cannot assert it, because it
    /// must stay total — so the check lives here and the sink fires it
    /// under `debug_assert!`, keeping the detector in test builds and
    /// the graceful degradation in release. Nothing today unbalances the
    /// stream, but nothing prevents it either: a future
    /// `precede(...).abandon(...)` would, and should fail loudly rather
    /// than yield a quietly odd tree.
    pub(crate) fn is_balanced(&self, root: SyntaxKind) -> bool {
        self.parents.is_empty()
            && matches!(
                self.children.as_slice(),
                [(_, NodeOrToken::Node(node))] if node.kind() == root
            )
    }

    /// Finish the tree, returning its root.
    ///
    /// Closes anything still open and, if what remains is not already a
    /// single `root` node, wraps it in one. Both fallbacks are
    /// unreachable for the sink's balanced stream — see
    /// [`GreenBuilder::is_balanced`], which the sink asserts — and exist
    /// so that a malformed stream degrades to an odd tree rather than a
    /// panic. They retain every element either way, so the result stays
    /// lossless.
    pub(crate) fn finish(mut self, root: SyntaxKind) -> GreenNode {
        while !self.parents.is_empty() {
            self.finish_node();
        }
        if self.children.len() == 1 {
            match self.children.pop() {
                Some((_, NodeOrToken::Node(node))) if node.kind() == root => return node,
                Some(orphan) => self.children.push(orphan),
                None => {}
            }
        }
        self.node(root, 0).1
    }

    /// Build the node spanning `children[first_child..]`, interning it if
    /// it is eligible.
    ///
    /// A node is eligible only if it is narrow enough *and* every child
    /// is itself interned — an un-interned child has no identity, so the
    /// parent has no key. That is the same invariant rowan maintains,
    /// and it means one wide node makes every ancestor un-interned too.
    #[inline]
    fn node(&mut self, kind: SyntaxKind, first_child: usize) -> (Id, GreenNode) {
        // `first_child` is recorded from `children.len()` when the node
        // opened, and `children` only ever shrinks back to an inner
        // node's mark, so it is always in range. Clamped once here
        // regardless: every path below slices or drains at it, and a
        // total function is worth one `min` on a cold branch.
        let first_child = first_child.min(self.children.len());
        let Some(key) = self.node_key(kind, first_child) else {
            return (NOT_INTERNED, self.build(kind, first_child));
        };
        let Self {
            nodes,
            children,
            next_id,
            ..
        } = self;
        match nodes.entry(key) {
            Entry::Occupied(hit) => {
                children.truncate(first_child);
                hit.get().clone()
            }
            Entry::Vacant(slot) => {
                *next_id += 1;
                let node = GreenNode::new(
                    kind,
                    children.drain(first_child..).map(|(_, element)| element),
                );
                slot.insert((*next_id, node)).clone()
            }
        }
    }

    /// Interning key for the pending node, or `None` if it is not
    /// eligible.
    ///
    /// `first_child` is already clamped by [`GreenBuilder::node`].
    #[inline]
    fn node_key(&self, kind: SyntaxKind, first_child: usize) -> Option<NodeKey> {
        let pending = self.children.get(first_child..)?;
        if pending.len() > MAX_INTERNED_CHILDREN {
            return None;
        }
        let mut key = NodeKey {
            kind,
            len: pending.len() as u8,
            children: [NOT_INTERNED; MAX_INTERNED_CHILDREN],
        };
        for (slot, &(id, _)) in key.children.iter_mut().zip(pending) {
            if id == NOT_INTERNED {
                return None;
            }
            *slot = id;
        }
        Some(key)
    }

    /// Allocate the node, consuming its children.
    #[inline]
    fn build(&mut self, kind: SyntaxKind, first_child: usize) -> GreenNode {
        GreenNode::new(
            kind,
            self.children
                .drain(first_child..)
                .map(|(_, element)| element),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: SyntaxKind = SyntaxKind(0);
    const PAIR: SyntaxKind = SyntaxKind(1);
    const LEAF: SyntaxKind = SyntaxKind(2);
    const OP: SyntaxKind = SyntaxKind(3);

    fn text(node: &GreenNode) -> String {
        node.to_string()
    }

    #[test]
    fn builds_a_flat_tree_losslessly() {
        let mut b = GreenBuilder::new(0);
        b.start_node(ROOT);
        b.token(LEAF, "a");
        b.token(OP, "+");
        b.token(LEAF, "b");
        b.finish_node();
        let root = b.finish(ROOT);
        assert_eq!(root.kind(), ROOT);
        assert_eq!(text(&root), "a+b");
        assert_eq!(root.children().count(), 3);
    }

    #[test]
    fn identical_subtrees_are_shared() {
        let mut b = GreenBuilder::new(0);
        b.start_node(ROOT);
        for _ in 0..2 {
            b.start_node(PAIR);
            b.token(LEAF, "x");
            b.token(LEAF, "y");
            b.finish_node();
        }
        b.finish_node();
        let root = b.finish(ROOT);
        assert_eq!(text(&root), "xyxy");
        let mut kids = root.children();
        let (first, second) = (kids.next().unwrap(), kids.next().unwrap());
        let (NodeOrToken::Node(first), NodeOrToken::Node(second)) = (first, second) else {
            panic!("expected two child nodes");
        };
        assert!(
            std::ptr::eq(first, second),
            "structurally identical subtrees should be interned to one allocation"
        );
    }

    #[test]
    fn differing_subtrees_are_not_shared() {
        let mut b = GreenBuilder::new(0);
        b.start_node(ROOT);
        b.start_node(PAIR);
        b.token(LEAF, "x");
        b.finish_node();
        b.start_node(PAIR);
        b.token(LEAF, "y");
        b.finish_node();
        b.finish_node();
        let root = b.finish(ROOT);
        assert_eq!(text(&root), "xy");
        let mut kids = root.children();
        let (NodeOrToken::Node(first), NodeOrToken::Node(second)) =
            (kids.next().unwrap(), kids.next().unwrap())
        else {
            panic!("expected two child nodes");
        };
        assert!(!std::ptr::eq(first, second));
    }

    /// A node wider than the intern threshold has no identity, and
    /// neither can anything built above it. The tree must still be
    /// correct — only the sharing is given up.
    #[test]
    fn wide_nodes_are_built_without_interning() {
        let mut b = GreenBuilder::new(0);
        b.start_node(ROOT);
        for _ in 0..2 {
            b.start_node(PAIR);
            for c in ["a", "b", "c", "d"] {
                b.token(LEAF, c);
            }
            b.finish_node();
        }
        b.finish_node();
        let root = b.finish(ROOT);
        assert_eq!(text(&root), "abcdabcd");
        let mut kids = root.children();
        let (NodeOrToken::Node(first), NodeOrToken::Node(second)) =
            (kids.next().unwrap(), kids.next().unwrap())
        else {
            panic!("expected two child nodes");
        };
        assert!(!std::ptr::eq(first, second));
    }

    /// The shape from issue #22: `x+x+…+x`, a left-deep spine in which
    /// every node's first child is the whole tree built so far. The
    /// sink emits such a spine outermost-first, by resolving the
    /// `forward_parent` chain `precede` leaves behind, so this mirrors
    /// that order. Every level is distinct, so nothing may be shared,
    /// and the text must round-trip.
    ///
    /// `N` is kept modest because rowan renders and drops a tree
    /// recursively, so a unit test that built a 5,000-deep one would
    /// abort the test process rather than report a failure. Long spines
    /// are exercised end-to-end in `tests/chain_scaling.rs`, which
    /// leaks the tree instead of dropping it.
    #[test]
    fn left_deep_spine_round_trips() {
        const N: usize = 256;
        let mut b = GreenBuilder::new(0);
        b.start_node(ROOT);
        for _ in 0..N {
            b.start_node(PAIR);
        }
        b.token(LEAF, "x");
        for _ in 0..N {
            b.token(OP, "+");
            b.token(LEAF, "x");
            b.finish_node();
        }
        b.finish_node();
        let root = b.finish(ROOT);
        assert_eq!(text(&root), format!("x{}", "+x".repeat(N)));

        let mut depth = 0;
        let mut cursor: &rowan::GreenNodeData = &root;
        loop {
            depth += 1;
            let child = cursor.children().find_map(|c| match c {
                NodeOrToken::Node(n) => Some(n),
                NodeOrToken::Token(_) => None,
            });
            match child {
                Some(n) => cursor = n,
                None => break,
            }
        }
        assert_eq!(depth, N + 1, "one ROOT plus one PAIR per extension");
    }

    /// The sink's stream is balanced, so neither fallback in `finish`
    /// fires in practice. They still must not panic: "no panics on user
    /// input, ever" (CONSTITUTION.md, principle III).
    #[test]
    fn unbalanced_streams_do_not_panic() {
        let mut b = GreenBuilder::new(0);
        b.start_node(ROOT);
        b.start_node(PAIR);
        b.token(LEAF, "a");
        // Two nodes left open.
        let root = b.finish(ROOT);
        assert_eq!(root.kind(), ROOT);
        assert_eq!(text(&root), "a");

        let mut b = GreenBuilder::new(0);
        b.finish_node(); // close with nothing open
        b.token(LEAF, "a");
        b.token(LEAF, "b");
        let root = b.finish(ROOT);
        assert_eq!(root.kind(), ROOT);
        assert_eq!(text(&root), "ab");

        let root = GreenBuilder::new(0).finish(ROOT);
        assert_eq!(root.kind(), ROOT);
        assert_eq!(text(&root), "");
    }
}
