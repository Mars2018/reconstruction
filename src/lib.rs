extern crate abomonation;
#[macro_use] extern crate abomonation_derive;
extern crate timely;

pub mod operators;

use std::cmp::Ordering;

use timely::ExchangeData;

pub type Timestamp = u64;
pub type TraceId = u32;
pub type Degree = u32;

/// A sessionizable message.
///
/// Sessionizion requires two properties for each recorded message:
///
///    - a session identifier
///    - the log record timestamp
pub trait SessionizableMessage: ExchangeData {
//    type Timestamp: ExchangeData;

    fn time(&self) -> Timestamp;
    fn session(&self) -> &str;
}

pub trait SpanPosition {
    fn get_span_id(&self) -> &SpanId;
}

#[derive(Debug, Clone, Abomonation)]
pub struct MessagesForSession<M: SessionizableMessage> {
    pub session: String,
    pub messages: Vec<M>,
}

// Method to convert a Vec<Vec<u32>> indicating paths through a tree to a canonical
// representation of the tree
//
// the result is a sequence of degrees of a BFS traversal of the graph.
pub fn canonical_shape<S: AsRef<Vec<TraceId>>>(paths: &Vec<S>) -> Vec<Degree> {
    let mut position = vec![0; paths.len()];
    let mut degrees = vec![0];
    let mut offsets = vec![1]; // where do children start?

    if let Some(max_depth) = paths.iter().map(|p| p.as_ref().len()).max() {
        for depth in 0 .. max_depth {
            // advance each position based on its offset
            // ensure that the max degree of the associated node is at least as high as it should be.
            for index in 0..paths.len() {
                if paths[index].as_ref().len() > depth {
                    if depth > 0 {
                        position[index] = (offsets[position[index]] + paths[index].as_ref()[depth-1]) as usize;
                    }

                    degrees[position[index]] = ::std::cmp::max(degrees[position[index]], paths[index].as_ref()[depth] + 1);
                }
            }

            // add zeros and transform degrees to offsets.
            let mut last = 0;
            for &x in &degrees { last += x as usize; }

            while degrees.len() <= last {
                degrees.push(0);
                offsets.push(0);
            }

            for i in 1..degrees.len() {
                offsets[i] = offsets[i-1] + degrees[i-1];
            }

        }
    }

    return degrees;
}

/// Describes where a span is located within the trace tree.
///
/// A hierarchical identifier is returned (e.g. `1-2-0`) which reflects the level of nesting for a
/// given span and specifies a path from the root span to the given span node.  Concretely, the
/// span ID is a vector of non-negative integers where each index corresponds to the positions
/// encountered while traversing down the tree.  In this example, the spans are nested in three
/// levels and the index `1` indicates that there were two _root_ spans with three nodes below it
/// (hence `2` as the next index) and the particular span being referred to is the first of these
/// children.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Abomonation)]
pub struct SpanId(pub Vec<TraceId>);

impl AsRef<Vec<TraceId>> for SpanId {
    fn as_ref(&self) -> &Vec<TraceId> {
        &self.0
    }
}

impl PartialOrd for SpanId {
    fn partial_cmp(&self, other: &SpanId) -> Option<Ordering> {
        let mut it1 = self.0.iter();
        let mut it2 = other.0.iter();
        loop {
            match (it1.next(), it2.next()) {
                (Some(p1), Some(p2)) => {
                    if p1 != p2 { return None } else { continue }
                }
                (None, Some(_)) => return Some(Ordering::Less),
                (Some(_), None) => return Some(Ordering::Greater),
                (None, None) => return Some(Ordering::Equal),
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{canonical_shape, SpanId};
    use std::cmp::Ordering;

    #[test]
    fn test_tree_shape() {
        assert_eq!(canonical_shape(&vec![SpanId(vec![0])]),
                   vec![1,0]);
        assert_eq!(canonical_shape(&vec![SpanId(vec![1])]),
                   vec![2,0,0]);
        assert_eq!(canonical_shape(&vec![SpanId(vec![0, 1])]),
                   vec![1,2,0,0]);
        assert_eq!(canonical_shape(&vec![SpanId(vec![2, 1, 3]), SpanId(vec![3])]),
                   vec![4,0,0,2,0,0,4,0,0,0,0]);
    }

    #[test]
    fn test_span_id_ordering() {
        let id = SpanId(vec![1, 0, 1]);
        assert_eq!(SpanId(vec![0]).partial_cmp(&id), None);
        assert_eq!(SpanId(vec![1]).partial_cmp(&id), Some(Ordering::Less));
        assert_eq!(SpanId(vec![1, 0]).partial_cmp(&id), Some(Ordering::Less));
        assert_eq!(SpanId(vec![1, 0, 0]).partial_cmp(&id), None);
        assert_eq!(SpanId(vec![1, 0, 1]).partial_cmp(&id), Some(Ordering::Equal));
        assert_eq!(SpanId(vec![1, 0, 2]).partial_cmp(&id), None);
        assert_eq!(SpanId(vec![1, 0, 1, 0]).partial_cmp(&id), Some(Ordering::Greater));
        assert_eq!(SpanId(vec![1, 0, 1, 0, 0]).partial_cmp(&id), Some(Ordering::Greater));
        assert_eq!(SpanId(vec![1, 0, 1, 0, 1]).partial_cmp(&id), Some(Ordering::Greater));
    }
}
