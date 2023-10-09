/*--- path: a.js ---*/
module.exports = function () {

};

/*--- path: b.js ---*/
let mod = require("./a.js");

/**/ mod;
//   ^ defined: 2, 7

class Quux {
    bar() {
    }
}
