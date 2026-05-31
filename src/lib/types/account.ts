import type { NonsModel } from "./base";

export interface Account extends NonsModel {
  token: string;
  avatarUrl: string;
  isLoggedIn: boolean;
  key: string;
}
