function f() {
  if(true) {
    return { v: 42 };
  }
}

  f().v;
//^ defined: 1
//    ^ defined: 3

export {};
