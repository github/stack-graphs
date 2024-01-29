class Node3:
    value = 3
class Node2:
    value = 2
    next = Node3
class Node1:
    value = 1
    next = Node2

def linked_list_search(l, item):
    node = l
    while node:
        if node.value == item:
            return node
            #      ^ defined: 10, 11, 16
        node = node.next

linked_list_search(Node1, 5).value
#                  ^ defined: 6
