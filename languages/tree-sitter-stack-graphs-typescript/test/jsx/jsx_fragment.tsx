let x = 1;

// Flow Around

const el = <></>;

/**/ x;
//   ^ defined: 1

// Children
(<foo><bar>{x}</bar></foo>);
//          ^ defined: 1
