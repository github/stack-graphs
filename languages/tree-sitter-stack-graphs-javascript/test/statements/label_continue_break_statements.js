let x = 1;

outer: while (t) {
    while (t) {
        break outer;
        //    ^ defined: 3
    }
    continue outer;
    //       ^ defined: 3
}

// Flow around

/**/ x;
//   ^ defined: 1