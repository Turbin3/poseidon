import {
  Account,
  Pubkey,
  Result,
  u64,
  Signer,
  Vec,
  String,
} from "@solanaturbine/poseidon";

export default class FavoritesProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  setFavorites(
    owner: Signer,
    number: u64,
    color: String<50>,
    hobbies: Vec<String<50>, 5>,
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
  color: String<50>;
  hobbies: Vec<String<50>, 5>;
}
