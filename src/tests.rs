#[cfg(test)]
mod tests {
    use crate::dheap::*;

    #[test]
    fn create_dheap() {
        let heap: DHeap<i32> = DHeap::with_capacity(16);
        assert_eq!(heap.size(), 1);
    }

    #[test]
    fn allocate_and_deallocate() {
        let heap: DHeap<i32> = DHeap::with_capacity(16);
        let val = 42;

        let dbox = heap.safe_new(val).unwrap();
        assert_eq!(heap.size(), 2);
        assert_eq!(*dbox, val);

        let inner_val = dbox.into_inner();
        assert_eq!(inner_val, val);
        assert_eq!(heap.size(), 2);
    }

    #[test]
    fn multiple_allocations() {
        let heap: DHeap<i32> = DHeap::with_capacity(16);
        let vals = vec![1, 2, 3, 4, 5];

        let mut dboxes = Vec::new();
        for val in &vals {
            dboxes.push(heap.safe_new(*val).unwrap());
        }

        for (i, dbox) in dboxes.iter().enumerate() {
            assert_eq!(**dbox, vals[i]);
        }

        assert_eq!(heap.size(), 6);

        dboxes.clear();

        assert_eq!(heap.size(), 6);

        for val in &vals {
            dboxes.push(heap.safe_new(*val).unwrap());
        }

        for (i, dbox) in dboxes.iter().enumerate() {
            assert_eq!(**dbox, vals[i]);
        }

        assert_eq!(heap.size(), 6);

        for dbox in dboxes {
            dbox.into_inner();
        }

        assert_eq!(heap.size(), 6);
    }

    struct ListNode<'a> {
        value: i32,
        next: Option<DBox<'a, ListNode<'a>>>,
    }

    #[test]
    fn linked_list() {
        let heap = DHeap::with_capacity(16);

        let mut prev_node: Option<DBox<ListNode>> = None;

        for value in 0..10 {
            println!("Adding {}", value);
            let node = heap
                .safe_new(ListNode {
                    value,
                    next: prev_node.map(|node| heap.safe_new(node.into_inner()).unwrap()),
                })
                .unwrap();
            prev_node = Some(node);
        }

        for _ in 0..4 {
            if let Some(node) = prev_node {
                println!("Popping {}", node.value);
                prev_node = node.into_inner().next;
            }
        }

        for value in 0..10 {
            println!("Adding {}", value);
            let node = heap
                .safe_new(ListNode {
                    value,
                    next: prev_node.map(|node| heap.safe_new(node.into_inner()).unwrap()),
                })
                .unwrap();
            prev_node = Some(node);
        }

        while let Some(node) = prev_node {
            println!("Popping {}", node.value);
            prev_node = node.into_inner().next;
        }

        println!("Final Size {}", heap.size());
    }
}
