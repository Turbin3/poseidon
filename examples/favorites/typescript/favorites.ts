import {
  Account,
  Pubkey,
  Result,
  u64,
  Signer,
  Vec,
  Str,
} from "@solanaturbine/poseidon";

export default class FavoritesProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  setFavorites(
    owner: Signer,
    number: u64,
    color: Str<50>,
    hobbies: Vec<Str<50>, 5>,
    favorites: Favorites,
  ): Result {
    favorites.derive(["favorites", owner.key]).initIfNeeded(owner);

    favorites.number = number;
    favorites.color = color;
    favorites.hobbies = hobbies;
  }
}

export interface Favorites extends Account {
  number: u64;
  color: Str<50>;
  hobbies: Vec<Str<50>, 5>;
}