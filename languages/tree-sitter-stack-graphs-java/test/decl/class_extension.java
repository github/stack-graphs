class String {
    public int length() {}
};

class HashMap<K, V> {
    public Set<MapEntry<K, V>> entrySet() {}
    //                  ^ defined: 5
}

class LRUCache<K, V>
    extends HashMap<K, V> {
    //      ^ defined: 5
    //              ^ defined: 10
    //                 ^ defined: 10
}
