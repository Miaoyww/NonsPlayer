let _showFullPlayer = $state(false);
let _playerMetaShow = $state(true);
let _pureLyricMode = $state(false);

export const playerUI = {
	get showFullPlayer() { return _showFullPlayer; },
	set showFullPlayer(v: boolean) { _showFullPlayer = v; },
	get playerMetaShow() { return _playerMetaShow; },
	set playerMetaShow(v: boolean) { _playerMetaShow = v; },
	get pureLyricMode() { return _pureLyricMode; },
	set pureLyricMode(v: boolean) { _pureLyricMode = v; },
};
