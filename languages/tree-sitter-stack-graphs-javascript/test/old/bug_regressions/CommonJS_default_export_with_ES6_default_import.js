/*--- path: a.js ---*/

const f = 5;

module.exports = f;

/*--- path: b.js ---*/

import g from './a.js';

/**/ g;
//   ^ defined: 3, 5, 9
