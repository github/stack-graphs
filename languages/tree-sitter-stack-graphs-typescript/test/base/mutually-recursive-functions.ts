function odd(x) {
  return x > 1 ? even(x - 1) : x === 1;
  //             ^ defined: 5
}
function even(x) {
  return x > 1 ? odd(x - 1) : x === 0;
  //             ^ defined: 1
}

export {};
