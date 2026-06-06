/** Result from loginQrKey — contains the unique key for QR code login. */
export interface QrKeyResult {
  unikey: string;
}

/** Full lyric data returned by fetchNeteaseLyric. */
export interface NeteaseLyricResult {
  lrc: string;
  yrc: string;
  tlyric: string;
  romalrc: string;
  ytlrc: string;
  yromalrc: string;
}
