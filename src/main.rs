use newick_parser::newick::{node_to_newick_no_lengths, newick_to_tree, NewickParser, Rule};
use newick_parser::node::{FlatTree, TraversalOrder};
use pest::Parser;
use std::env;
use std::fs;

/// The updated SPR function with debug print statements.
pub fn spr(
    flat_tree: &mut FlatTree,
    donor: usize,
    recipient: usize,
    time: f64,
) {
    // Get initial state
    let donor_parent = flat_tree[donor]
        .parent
        .expect("The donor node should not be the root");
    let recipient_parent = flat_tree[recipient]
        .parent
        .expect("The recipient node should not be the root");

    let recipient_sibling = if flat_tree[recipient_parent].left_child.unwrap() == recipient {
        flat_tree[recipient_parent].right_child.unwrap()
    } else {
        flat_tree[recipient_parent].left_child.unwrap()
    };

    // Check if recipient's parent is the root
    if flat_tree[recipient_parent].parent.is_none() {
        // The recipient's sibling becomes the new root.
        flat_tree[recipient_sibling].parent = None;
        flat_tree.root = recipient_sibling;
        // Reassign the recipient's parent: attach it under the donor's parent.
        flat_tree[recipient_parent].parent = Some(donor_parent);

        // Update the child pointer in recipient_parent: replace the recipient's parent's sister with the donor.
        if flat_tree[recipient_parent].left_child.unwrap() == recipient {
            flat_tree[recipient_parent].right_child = Some(donor);
        } else {
            flat_tree[recipient_parent].left_child = Some(donor);
        }
        flat_tree[recipient_parent].depth = Some(time);

        // Update donor_parent so that its child pointer now points to recipient_parent.
        if flat_tree[donor_parent].left_child.unwrap() == donor {
            flat_tree[donor_parent].left_child = Some(recipient_parent);
        } else {
            flat_tree[donor_parent].right_child = Some(recipient_parent);
        }
        // Finally, attach the donor under recipient_parent.
        flat_tree[donor].parent = Some(recipient_parent);
    } else {
        // Normal case: recipient_parent is not the root.
        let recipient_grandparent = flat_tree[recipient_parent].parent;

        flat_tree[recipient_parent].parent = Some(donor_parent);
        if flat_tree[recipient_parent].left_child.unwrap() == recipient {
            flat_tree[recipient_parent].left_child = Some(donor);
        } else {
            flat_tree[recipient_parent].right_child = Some(donor);
        }
        flat_tree[recipient_parent].depth = Some(time);

        if let Some(gp) = recipient_grandparent {
            if flat_tree[gp].left_child.unwrap() == recipient_parent {
                flat_tree[gp].left_child = Some(recipient_sibling);
            } else {
                flat_tree[gp].right_child = Some(recipient_sibling);
            }
            flat_tree[recipient_sibling].parent = Some(gp);
                }
        if flat_tree[donor_parent].left_child.unwrap() == donor {
            flat_tree[donor_parent].left_child = Some(recipient_parent);
        } else {
            flat_tree[donor_parent].right_child = Some(recipient_parent);
        }
        flat_tree[donor].parent = Some(recipient_parent);
    }

}

fn main() {
    // Expect four arguments: tree file, donor name, recipient name, and output file.
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!(
            "Usage: {} <tree_file> <donor> <recipient> <output_file>",
            args[0]
        );
        return;
    }

    let tree_file = &args[1];
    let donor_name = &args[2];
    let recipient_name = &args[3];
    let output_file = &args[4];

    // Read and sanitize the tree (expecting Newick format ending with a semicolon)
    let tree_str = fs::read_to_string(tree_file).expect("Failed to read tree file");
    let sanitized = tree_str.trim();
    let trees: Vec<String> = sanitized
        .split(';')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(format!("{};", s))
            }
        })
        .collect();

    if trees.is_empty() {
        eprintln!("No tree found in file.");
        return;
    }

    // Use the first tree found in the file.
    let tree_newick = &trees[0];
    let pairs = NewickParser::parse(Rule::newick, tree_newick)
        .expect("Failed to parse Newick tree");
    let mut node_tree = newick_to_tree(pairs.into_iter().next().unwrap())
        .pop()
        .expect("No tree produced");
    let mut flat_tree = node_tree.to_flat_tree();

    // Locate donor and recipient nodes by name.
    let donor_index = flat_tree
        .iter(TraversalOrder::PreOrder)
        .position(|node| node.name == *donor_name)
        .unwrap_or_else(|| panic!("Donor '{}' not found in tree", donor_name));
    let recipient_index = flat_tree
        .iter(TraversalOrder::PreOrder)
        .position(|node| node.name == *recipient_name)
        .unwrap_or_else(|| panic!("Recipient '{}' not found in tree", recipient_name));

    // Prevent invalid moves: donor must not be a descendant of the recipient.
    let mut current = flat_tree[donor_index].parent;
    while let Some(parent) = current {
        if parent == recipient_index {
            eprintln!(
                "Invalid SPR: donor '{}' is a descendant of recipient '{}'",
                donor_name, recipient_name
            );
            std::process::exit(1);
        }
        current = flat_tree[parent].parent;
    }
    // Helper function to format Option<T> as a string.
    fn fmt_option<T: std::fmt::Display>(opt: Option<T>) -> String {
        match opt {
            Some(val) => format!("{}", val),
            None => String::from("None"),
        }
    }


    // Apply the SPR event with a fixed time (0.5).
    spr(&mut flat_tree, donor_index, recipient_index, 0.5);

    // Update the root in case the topology has changed.
    let root_index = flat_tree.nodes
        .iter()
        .position(|node| node.parent.is_none())
        .expect("No root found in the tree");
    flat_tree.root = root_index;

    // Reconstruct the node tree and update branch lengths based on node depths.
    let gene_tree = flat_tree.to_node();

    // Convert the modified tree to Newick format and write it to the output file.
    let newick = node_to_newick_no_lengths(&gene_tree) + ";";
    fs::write(output_file, newick).expect("Failed to write gene tree to file");
}
