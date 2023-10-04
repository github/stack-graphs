/* --- path: package.json --- */

{
    "name": "@acme/foo",
    "version": "1.0",
    "main": "./api"
}

/* --- path: core.js --- */

export let x;

  x;
//^ defined: 11

/* --- path: api.js --- */

export let x;

  x;
//^ defined: 18
