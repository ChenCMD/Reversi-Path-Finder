// Generate a random 36-bit integer
function rand36() {
  return (BigInt(Math.floor(Math.random() * 2 ** 18)) << 18n) |
         BigInt(Math.floor(Math.random() * 2 ** 18));
}

// Convert a 36-bit number to a 12-digit octal string
function toOctal36(n) {
  return n.toString(8).padStart(12, '0');
}

// Generate random 36-bit integer with given bit-on probability
function random36(prob = 0.5) {
  let n = 0n;
  for (let i = 0n; i < 36n; i++) {
    if (Math.random() < prob) n |= (1n << i);
  }
  return n;
}

function shuffle(arr) {
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

// Generate disjoint placement masks that (1) cover all 36 cells and (2) have equal counts.
function generateBalancedPlacementMasks() {
  const positions = shuffle([...Array(36).keys()]);

  let black = 0n;
  let white = 0n;
  for (let i = 0; i < positions.length; i++) {
    const bit = 1n << BigInt(positions[i]);
    if (i < 18) {
      black |= bit;
    } else {
      white |= bit;
    }
  }

  return { black, white };
}

const mask36 = (1n << 36n) - 1n;

// 1. random 36-bit number n
const n = rand36();

// 2. derive z (bits that are zero) and o (bits that are one)
const o = n;
const z = (~n) & mask36;

// 3. balanced, complementary placement masks
const { black: b, white: w } = generateBalancedPlacementMasks();

// 4. print in fixed order
console.log("black bitboard       :", toOctal36(z));
console.log("white bitboard       :", toOctal36(o));
console.log("black placement mask :", toOctal36(b));
console.log("white placement mask :", toOctal36(w));
