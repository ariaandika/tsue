#[derive(Debug)]
pub enum Tree {
    Branch(Branch),
    Leaf(Leaf),
}

#[derive(Debug)]
pub struct Branch {
    left: Option<Box<Tree>>,
    right: Option<Box<Tree>>,
}

#[derive(Debug)]
pub struct Leaf {
    byte: u8,
}

impl Leaf {
    pub fn new(byte: u8) -> Self {
        Self { byte }
    }

    pub fn byte(&self) -> u8 {
        self.byte
    }
}

impl Branch {
    pub fn new() -> Self {
        Self {
            left: None,
            right: None,
        }
    }
}

impl Tree {
    pub fn new() -> Self {
        Self::Branch(Branch {
            left: None,
            right: None,
        })
    }

    /// Get or create new left branch.
    ///
    /// Current tree must be a [`Tree::Branch`].
    pub fn left_branch(&mut self) -> &mut Tree {
        let Tree::Branch(branch) = self else {
            panic!("conflicting leaf")
        };
        let left = branch
            .left
            .get_or_insert_with(|| Box::new(Self::Branch(Branch::new())));
        assert!(left.is_branch());
        left
    }

    /// Get or create new right branch.
    ///
    /// Current tree must be a [`Tree::Branch`].
    pub fn right_branch(&mut self) -> &mut Tree {
        let Tree::Branch(branch) = self else {
            panic!("conflicting leaf")
        };
        let right = branch
            .right
            .get_or_insert_with(|| Box::new(Self::Branch(Branch::new())));
        assert!(right.is_branch());
        right
    }

    /// Replace current tree into a leaf.
    ///
    /// Current tree must be an empty [`Tree::Branch`].
    pub fn replace_as_leaf(&mut self, leaf: Leaf) {
        let Self::Branch(branch) = self else {
            panic!("conflicting leaf");
        };
        assert!(
            branch.left.is_none() && branch.right.is_none(),
            "conflicting leaf"
        );
        *self = Self::Leaf(leaf);
    }

    pub fn is_branch(&self) -> bool {
        matches!(self, Self::Branch(..))
    }

    pub fn get(&self, bits: &[bool]) -> &Leaf {
        let mut current = self;
        for bit in bits {
            current = match bit {
                false => current.assert_left_branch(),
                true => current.assert_right_branch(),
            }
        }
        current.assert_leaf()
    }

    /// Assert and returns left branch.
    pub fn assert_left_branch(&self) -> &Tree {
        let Self::Branch(branch) = self else {
            panic!("conflicting leaf")
        };
        branch.left.as_ref().expect("cannot get left branch")
    }

    /// Assert and returns right branch.
    pub fn assert_right_branch(&self) -> &Tree {
        let Self::Branch(branch) = self else {
            panic!("conflicting leaf")
        };
        branch.right.as_ref().expect("cannot get right branch")
    }

    /// Assert and returns current tree as a leaf.
    pub fn assert_leaf(&self) -> &Leaf {
        let Self::Leaf(leaf) = self else {
            panic!("expected leaf, found branch")
        };
        leaf
    }

    #[allow(unused)]
    pub fn debug_print(&self, buffer: &mut Vec<u8>) {
        match self {
            Tree::Branch(branch) => {
                buffer.push(b'0');
                if let Some(left_tree) = branch.left.as_deref() {
                    left_tree.debug_print(buffer);
                }
                *buffer.last_mut().unwrap() = b'1';
                if let Some(right_tree) = branch.right.as_deref() {
                    right_tree.debug_print(buffer);
                }
                buffer.remove(buffer.len() - 1);
            }
            Tree::Leaf(leaf) => {
                print!("{: <32}", str::from_utf8(buffer).unwrap());
                println!("{}", leaf.byte);
            }
        }
    }
}
