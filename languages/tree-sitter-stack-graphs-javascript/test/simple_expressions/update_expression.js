let x = 1;

// Flow in
/**/ x++;
//   ^ defined: 1

// Flow around and update

/**/ x;
//   ^ defined: 1, 5