// Implementation of Myers' online approximate string matching algorithm [1],
// with additional optimizations suggested by [2]. See also alternate
// explanation in [3].
//
// This has O((k/w) * n) expected-time where `n` is the length of the
// text, `k` is the maximum number of errors allowed (always <= the pattern
// length) and `w` is the word size, which is 64.
//
// As far as I am aware, there aren't any online algorithms which are
// significantly better for a wide range of input parameters. The problem can be
// solved faster using "filter then verify" approaches which first filter out
// regions of the text that cannot match using a "cheap" check and then verify
// the remaining potential matches. The verify step requires an algorithm such
// as this one however.
//
// The algorithm's approach is essentially to optimize the classic dynamic
// programming solution to the problem by computing columns of the matrix in
// word-sized chunks (ie. dealing with 32 chars of the pattern at a time) and
// avoiding calculating regions of the matrix where the minimum error count is
// guaranteed to exceed the input threshold.
//
// The paper consists of two parts, the first describes the core algorithm for
// matching patterns <= the size of a word (implemented by `advanceBlock` here).
// The second uses the core algorithm as part of a larger block-based algorithm
// to handle longer patterns.
//
// [1] G. Myers, “A Fast Bit-Vector Algorithm for Approximate String Matching
// Based on Dynamic Programming,” vol. 46, no. 3, pp. 395–415, 1999.
//
// [2] Šošić, M. (2014). An simd dynamic programming c/c++ library (Doctoral
// dissertation, Fakultet Elektrotehnike i računarstva, Sveučilište u Zagrebu).
//
// [3] Heikki Hyyrö (2001) Explaining and Extending the Bit-parallel Approximate
// String Matching Algorithm of Myers, Technical report, University of Tampere, Finland.

mod wasm;

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
struct Match {
    start: usize,
    end: usize,
    errors: usize,
}

type BlockWord = u64;

// Number of characters of the pattern represented by each block.
const BLOCK_LEN: usize = 64;

#[derive(Clone, Debug)]
struct Block {
    // Bit flags indicating whether each row in this block has one more error
    // than the row above.
    plus_v: BlockWord,

    // Bit flags indicating whether each row in this block has one less error
    // than the row above.
    minus_v: BlockWord,

    // Mask with one bit set indicating which is the last used position in this
    // block.
    last_row_mask: BlockWord,

    score: i32,
}

fn one_if_not_zero<T: PartialEq + Default>(n: T) -> i32 {
    if n != Default::default() {
        1
    } else {
        0
    }
}

fn reverse(chars: &[u16]) -> Vec<u16> {
    chars.iter().rev().cloned().collect()
}

fn find_match_starts(text: &[u16], pattern: &[u16], matches: &mut Vec<Match>) {
    let pat_rev = reverse(pattern);

    // TODO - Re-use the pattern generated for `find_match_ends`.
    let pattern_bits = PatternBits::new(&pat_rev);

    for m in matches.iter_mut() {
        // Find start of each match by reversing the pattern and matching segment
        // of text and searching for an approx match with the same number of
        // errors.
        let min_start = 0.max(m.end as i32 - pattern.len() as i32 - m.errors as i32) as usize;

        // TODO - Only generate this once or, better yet, use an iterator that reverses
        // over the text.
        let text_rev = reverse(&text[min_start..m.end]);

        // If there are multiple possible start points, choose the one that
        // maximizes the length of the match.
        let match_ends = find_match_ends(&text_rev, &pat_rev, m.errors, &pattern_bits);
        let mut start = m.end;

        for rm in match_ends {
            if m.end - rm.end < start {
                start = m.end - rm.end;
            }
        }

        m.start = start;
    }
}

/// Block calculation step of the algorithm.
///
/// From Fig 8. on p. 408 of [1].
///
/// block - Data for the current block
/// pattern_match_bits -
///   Bit flags indicating which positions within the current block of the pattern
///   match the current character of the text
/// h_in - Horizontal input delta (1, 0 or -1)
///
/// Returns horizontal output delta (1, 0 or -1)
fn advance_block(block: &mut Block, pattern_match_bits: BlockWord, h_in: i32) -> i32 {
    let p_v = block.plus_v;
    let m_v = block.minus_v;

    let h_in_negative = if h_in < 0 { 1 } else { 0 };

    let eq = pattern_match_bits | h_in_negative;

    // Step 1: Compute horizontal deltas.
    let x_v = eq | m_v;
    let x_h = (((eq & p_v).overflowing_add(p_v).0) ^ p_v) | eq;

    let mut p_h = m_v | !(x_h | p_v);
    let mut m_h = p_v & x_h;

    // Step 2: Update score (value of last row of this block).
    let h_out =
        one_if_not_zero(p_h & block.last_row_mask) - one_if_not_zero(m_h & block.last_row_mask);

    // Step 3: Update vertical deltas for use when processing next char.
    p_h <<= 1;
    m_h <<= 1;

    m_h |= h_in_negative;
    p_h |= one_if_not_zero(h_in) as BlockWord - h_in_negative; // Set p_h[0] if h_in > 0

    block.plus_v = m_h | !(x_v | p_h);
    block.minus_v = p_h & x_v;

    h_out
}

struct PatternBits {
    ascii: Vec<Option<Vec<BlockWord>>>,
    nonascii: HashMap<u16, Vec<BlockWord>>,
    zero: Vec<BlockWord>,
}

fn reverse_blocks(blocks: &mut Vec<BlockWord>) {
    blocks.reverse();
    for block in blocks.iter_mut() {
        *block = block.reverse_bits();
    }
}

impl<'a> PatternBits {
    fn new(pattern: &[u16]) -> PatternBits {
        // Number of blocks required by this pattern.
        let block_count = (pattern.len() + BLOCK_LEN - 1) / BLOCK_LEN;

        // Dummy match bit vector for chars in the text which do not occur in the pattern.
        let zero_bits = vec![0; block_count];

        // Map of non-ASCII UTF-16 character code to bit vector indicating positions in the
        // pattern that equal that character.
        let mut nonascii_match_bits: HashMap<u16, Vec<BlockWord>> = HashMap::new();

        // Map of ASCII character code to bit vector indicating positions in the
        // pattern that equal that character.
        let mut ascii_match_bits = vec![None; 256];

        // For each unique character in the pattern generate a bit vector indicating
        // the positions where it occurs in the pattern.
        for ch in pattern.iter() {
            // Check if we've already seen this char.
            if let Some(entry) = ascii_match_bits.get(*ch as usize) {
                if entry.is_some() {
                    continue;
                }
            } else if nonascii_match_bits.get(ch).is_some() {
                continue;
            }

            let mut match_bits: Vec<BlockWord> = vec![0; block_count];

            for (b, bits) in match_bits.iter_mut().enumerate() {
                // Set all the bits where the pattern matches the current char (ch).
                // For indexes beyond the end of the pattern, always set the bit as
                // if the pattern contained a wildcard char in that position.
                for r in 0..BLOCK_LEN {
                    let idx = b * BLOCK_LEN + r;
                    if idx >= pattern.len() {
                        continue;
                    }

                    if pattern[idx] == *ch {
                        *bits |= 1 << r;
                    }
                }
            }

            if let Some(entry) = ascii_match_bits.get_mut(*ch as usize) {
                *entry = Some(match_bits);
            } else {
                nonascii_match_bits.insert(*ch, match_bits);
            }
        }

        PatternBits {
            ascii: ascii_match_bits,
            nonascii: nonascii_match_bits,
            zero: zero_bits,
        }
    }

    fn lookup(&'a self, char_code: u16) -> &'a Vec<BlockWord> {
        match self.ascii.get(char_code as usize) {
            Some(maybe_char) => match maybe_char {
                Some(blocks) => &blocks,
                None => &self.zero,
            },
            None => self.nonascii.get(&char_code).unwrap_or(&self.zero),
        }
    }

    fn reverse(&mut self) {
        // Reverse all bit vectors in `self.ascii`
        for v in self.ascii.iter_mut() {
            if let Some(vec) = v {
                reverse_blocks(vec);
            }
        }

        // Reverse all bit vectors in `self.nonascii`
        for (_, block) in self.nonascii.iter_mut() {
            reverse_blocks(block);
        }
    }
}

fn find_match_ends(
    text: &[u16],
    pattern: &[u16],
    max_errors: usize,
    pattern_bits: &PatternBits,
) -> Vec<Match> {
    if pattern.is_empty() {
        return Vec::new();
    }

    // Clamp error count so we can rely on `max_errors` and `pattern.len()`
    // rows being in the same block below.
    let mut max_errors = max_errors.min(pattern.len()) as i32;

    let mut matches = Vec::new();

    // Number of blocks required by this pattern.
    let block_count = (pattern.len() + BLOCK_LEN - 1) / BLOCK_LEN;

    // Index of last-active block level in the column.
    let mut y = 0.max((max_errors as f32 / (BLOCK_LEN as f32)).ceil() as i32 - 1) as usize;

    // Data for the current column of the error count table.
    let mut blocks: Vec<Block> = Vec::with_capacity(block_count);
    for b in 0..block_count {
        blocks.push(Block {
            plus_v: !0,
            minus_v: 0,
            last_row_mask: if b == block_count - 1 {
                1 << ((pattern.len() - 1) % BLOCK_LEN)
            } else {
                1 << (BLOCK_LEN - 1)
            },
            score: if b == block_count - 1 {
                pattern.len()
            } else {
                (b + 1) * BLOCK_LEN
            } as i32,
        });
    }

    let mut scores = Vec::new();

    // Process each char of the text, computing the error count for `w` chars
    // of the pattern at a time.
    for (j, char_code) in text.iter().enumerate() {
        let match_bits = pattern_bits.lookup(*char_code);

        // Calculate error count for blocks that we definitely have to process
        // for this column.
        let mut carry = 0;
        for b in 0..=y {
            carry = advance_block(&mut blocks[b], match_bits[b], carry);
            blocks[b].score += carry;
        }

        // Check if we also need to compute an additional block, or if we can
        // reduce the number of blocks processed for the next column.
        if blocks[y].score - carry <= max_errors
            && y < (block_count - 1)
            && ((match_bits[y + 1] & 1 != 0) || carry < 0)
        {
            // Error count for bottom block is under threshold. Increase the number
            // of blocks processed for this column and the next by one.
            y += 1;

            blocks[y].plus_v = !0;
            blocks[y].minus_v = 0;

            let max_block_score = if y == (block_count - 1) {
                pattern.len() % BLOCK_LEN
            } else {
                BLOCK_LEN
            };
            blocks[y].score = blocks[y - 1].score + max_block_score as i32 - carry
                + advance_block(&mut blocks[y], match_bits[y], carry);
        } else {
            // Error count for bottom block exceeds threshold. Reduce the number
            // of blocks processed for the next column.
            while y > 0 && blocks[y].score >= max_errors + BLOCK_LEN as i32 {
                y -= 1;
            }
        }

        scores.push(blocks[y].score);

        // If error count is under threshold, report a match.
        if y == (block_count - 1) && blocks[y].score <= max_errors {
            if blocks[y].score < max_errors {
                // Discard any earlier, worse matches.
                matches.clear();
            }

            matches.push(Match {
                start: 0,
                end: j + 1,
                errors: blocks[y].score as usize,
            });

            // Because `search` only reports the matches with the lowest error
            // count, we can "ratchet down" the max error threshold whenever a
            // match is encountered and thereby save a small amount of work for
            // the remainder of the text.
            max_errors = blocks[y].score;
        }
    }

    matches
}

fn search_impl(text: &[u16], pattern: &[u16], max_errors: u32) -> Vec<Match> {
    let mut pattern_bits = PatternBits::new(&pattern);
    let mut matches = find_match_ends(text, pattern, max_errors as usize, &pattern_bits);
    find_match_starts(text, pattern, &mut matches);
    matches
}

#[cfg(test)]
mod tests {
    use crate::{search_impl, Match};

    fn utf16_str(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn match_str(text: &[u16], m: &Match) -> String {
        String::from_utf16(&text[m.start..m.end]).unwrap()
    }

    #[test]
    fn it_finds_short_pattern_in_short_text() {
        let text = utf16_str("hello world");
        let pattern = utf16_str("wrld");

        let matches = search_impl(&text, &pattern, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0],
            Match {
                start: 6,
                end: text.len(),
                errors: 1
            }
        );
    }

    #[test]
    fn it_finds_match_with_many_errors() {
        let text = utf16_str("The rain in Spain falls mainly on the plain");
        let pattern = utf16_str("rain in England falls");

        let matches = search_impl(&text, &pattern, pattern.len() as u32);

        assert_eq!(matches.len(), 1);
        assert_eq!(match_str(&text, &matches[0]), "rain in Spain falls");
    }

    #[test]
    fn it_finds_match_for_pattern_with_repeated_chars() {
        let text = utf16_str("some cases");
        let pattern = utf16_str("some cas");

        let matches = search_impl(&text, &pattern, 0);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0],
            Match {
                start: 0,
                end: pattern.len(),
                errors: 0,
            }
        );
    }

    #[test]
    fn it_finds_match_for_pattern_in_longer_string() {
        let text = utf16_str("Escaping double-quotes can be cumbersome in some cases such as writing regular expressions or defining a JSON object as a string literal");
        let pattern = utf16_str("reglar expressions");

        let matches = search_impl(&text, &pattern, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0],
            Match {
                start: 71,
                end: matches[0].start + pattern.len() + 1,
                errors: 1,
            }
        );
    }

    // #[test]
    fn it_finds_no_match() {
        let text = utf16_str("aaaaa");
        let pattern = utf16_str("bbb");
        let matches = search_impl(&text, &pattern, pattern.len() as u32);
        assert_eq!(matches.len(), 1);
    }

    // #[test]
    fn it_finds_no_match_for_long_pattern_in_long_text() {
        let text = utf16_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let pattern =
            utf16_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let matches = search_impl(&text, &pattern, pattern.len() as u32);
        assert_eq!(matches.len(), 67);
        assert_eq!(
            matches[0],
            Match {
                start: 0,
                end: text.len(),
                errors: pattern.len(),
            }
        )
    }

    #[test]
    fn it_finds_match_for_non_ascii_pattern() {
        let text = utf16_str("hello world 🙂");
        let pattern = utf16_str("world 🙂");
        let matches = search_impl(&text, &pattern, 0);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start, text.len() - pattern.len());
    }

    #[test]
    fn it_finds_match_for_long_pattern() {
        let text = utf16_str("Many years later, as he faced the firing squad, Colonel Aureliano Buendía was to remember that distant afternoon when his father took him to discover ice.");
        let pattern = text.clone();
        let matches = search_impl(&text, &pattern, 0);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].errors, 0);
        assert_eq!(matches[0].start, 0);
    }
}
