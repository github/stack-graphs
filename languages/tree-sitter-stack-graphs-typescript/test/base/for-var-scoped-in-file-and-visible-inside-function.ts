for(var x = 0; x < 42; x++) {
  let f = function() {
    x = 2;
  //^ defined: 1
  };
}

  x = 1;
//^ defined: 1

export {};
