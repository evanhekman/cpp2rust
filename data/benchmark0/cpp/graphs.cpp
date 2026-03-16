/*
graph node with non-owning neighbor links
- the c++ struct stores a string payload and a vector of raw pointers to adjacent nodes
- the raw Node* pointers do not express ownership, lifetime, or nullability clearly
- this design can work in c++, but it is easy to create dangling pointers if a pointed-to node is destroyed or moved
- a naive rust translation with Vec<&Node> usually fails because self-referential or cyclic graphs are hard to express with plain references
- a good rust translation typically uses indices into an arena, or shared ownership types like Rc<RefCell<Node>> when mutation and cycles are needed
*/

#include <string>
#include <vector>

struct Node {
    std::string name;
    std::vector<Node*> neighbors;
};