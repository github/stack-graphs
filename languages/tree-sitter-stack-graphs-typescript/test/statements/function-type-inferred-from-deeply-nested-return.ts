function f() {
  try {
    if(true) {
      return { v: 42 };
    }
  } finally {
    return { e: -1 };
  }
}

  f().v;
//^ defined: 1
//    ^ defined: 4

  f().e;
//^ defined: 1
//    ^ defined: 7

export {};
