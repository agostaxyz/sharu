// Mirrors crates/server/src/nick.rs: two CamelCase words, no digits.
const FRUITS = ['Almond', 'Apple', 'Apricot', 'Avocado', 'Banana', 'Berry', 'Blueberry', 'Cashew', 'Cherry', 'Coconut', 'Cranberry', 'Currant', 'Date', 'Fig', 'Grape', 'Guava', 'Hazel', 'Kiwi', 'Lemon', 'Lime', 'Lychee', 'Mango', 'Melon', 'Mulberry', 'Olive', 'Orange', 'Papaya', 'Peach', 'Pear', 'Pineapple', 'Plum', 'Pomelo', 'Quince', 'Raspberry', 'Tangerine'];
const ANIMALS = ['Badger', 'Beaver', 'Bison', 'Condor', 'Crane', 'Dolphin', 'Eagle', 'Falcon', 'Ferret', 'Finch', 'Fox', 'Gecko', 'Heron', 'Ibis', 'Jaguar', 'Koala', 'Lemur', 'Lynx', 'Magpie', 'Marmot', 'Newt', 'Ocelot', 'Okapi', 'Otter', 'Owl', 'Panda', 'Parrot', 'Pelican', 'Puffin', 'Quail', 'Raven', 'Robin', 'Seal', 'Sparrow', 'Swan', 'Tapir', 'Tiger', 'Toucan', 'Turtle', 'Walrus', 'Wombat', 'Yak', 'Zebra'];
const pick = <T,>(a: T[]): T => a[crypto.getRandomValues(new Uint32Array(1))[0]! % a.length]!;

export const defaultNickname = (): string => pick(FRUITS) + pick(ANIMALS);

/** 2-24 letters, digits, "_" or "-": no whitespace, control or bidi characters. */
export function sanitizeNickname(input: string): string | null {
  const s = input.trim();
  const len = [...s].length;
  return len >= 2 && len <= 24 && /^[\p{L}\p{N}_-]+$/u.test(s) ? s : null;
}
