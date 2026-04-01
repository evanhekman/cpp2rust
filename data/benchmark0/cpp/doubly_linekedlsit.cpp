/*
O(1) arbitrary node deletion in a doubly linked list (Task Queue)
- the c++ implementation returns a raw pointer to an internal node, allowing direct O(1) deletion but risking dangling pointers and violating exclusive mutability
- a naive rust implementation tries to replicate the exact memory graph using Rc<RefCell<Node>> for next and Weak<RefCell<Node>> for prev, resulting in massive boilerplate and runtime overhead
- a good rust implementation uses an Arena Allocator (a Vec<Node>) where prev and next are usize indices, and hands the caller a lightweight "Task ID" (index) instead of a direct memory reference
*/

#include <iostream>

struct Node {
    int task_id;
    Node* prev = nullptr;
    Node* next = nullptr;
};

class TaskQueue {
public:
    Node* head = nullptr;

    // 1. Returns a raw pointer directly into the middle of our data structure
    Node* addTask(int id) {
        Node* newNode = new Node{id};
        if (!head) {
            head = newNode;
            head->next = head;
            head->prev = head;
        } else {
            Node* tail = head->prev;
            tail->next = newNode;
            newNode->prev = tail;
            newNode->next = head;
            head->prev = newNode;
        }
        return newNode;
    }

    // 2. Deletes a node in O(1) time using ONLY the pointer provided by the user
    void cancelTask(Node* target) {
        if (!target) return;
        
        // Arbitrary mutation of neighbors without traversing from the root!
        target->prev->next = target->next;
        target->next->prev = target->prev;

        if (target == head) {
            head = (target->next == head) ? nullptr : target->next;
        }
        delete target;
    }
};

int main() {
    TaskQueue queue;
    
    // The caller takes ownership of raw pointers to internal nodes
    Node* task1 = queue.addTask(10);
    Node* task2 = queue.addTask(20);
    Node* task3 = queue.addTask(30);

    // NIGHTMARE 1: External, simultaneous mutation
    task2->task_id = 99; 

    // NIGHTMARE 2: Arbitrary deletion from an external reference
    queue.cancelTask(task2);

    // NIGHTMARE 3: Dangling pointer risk (C++ allows this, Rust forbids it)
    // task2->task_id = 100; // This would be a use-after-free segfault!

    return 0;
}