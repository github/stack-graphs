enum Level {
  LOW,
  MEDIUM,
  HIGH
}

class Supplier<T> {
  T get() {}
}

class App {
  boolean isSufficient(Supplier<Level> level_supplier) {
    switch (level_supplier.get()) {
      case LOW: return false;
      //   ^ defined: 2
      case MEDIUM: return false;
      //   ^ defined: 3
      case Level.HIGH: return true;
      //   ^ defined: 1
      //         ^ defined: 4
    }
  }
}
