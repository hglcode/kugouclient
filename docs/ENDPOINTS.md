# Endpoint catalog (ENDPOINTS)

271 endpoints in total, covering every module of the reference
implementation (KuGouMusicApi main\@5d6c9f7).

Regenerate with `cargo run -p kugouapi-cli -- endpoints --markdown`.

| endpoint id | module | method | path | origin / router | auth | note |
|---|---|---|---|---|---|---|
| `ai.recommend` | ai_recommend | POST | /recommend | router:songlistairec.kugou.com |  |  |
| `ai.recommend.song` | ai_recommend_song | POST | /concepts/v1/ai/recommend_song |  |  |  |
| `album` | album | POST | /v1/album | @http://kmr.service.kugou.com router:kmr.service.kugou.com |  |  |
| `album.detail` | album_detail | POST | /kmr/v2/albums | router:openapi.kugou.com |  |  |
| `album.dycover` | album_dycover | GET | /v2/album/audio | @https://kmrcdn.service.kugou.com |  |  |
| `album.shop` | album_shop | GET | /zhuanjidata/v3/album_shop_v2/get_classify_data |  |  |  |
| `album.songs` | album_songs | POST | /v1/album_audio/lite | router:openapi.kugou.com |  |  |
| `artist.albums` | artist_albums | POST | /kmr/v1/author/albums | router:openapi.kugou.com |  |  |
| `artist.audios` | artist_audios | POST | /kmr/v1/audio_group/author | @https://openapi.kugou.com router:openapi.kugou.com |  |  |
| `artist.audios.new` | artist_audios_new | GET | /openapi/kmr/v2/audio_group/author |  |  |  |
| `artist.detail` | artist_detail | POST | /kmr/v3/author | router:openapi.kugou.com |  |  |
| `artist.follow` | artist_follow | POST | /followservice/v3/follow_singer |  | yes |  |
| `artist.follow.newsongs` | artist_follow_newsongs | POST | /feed/v1/follow/newsong_album_list |  |  |  |
| `artist.honour` | artist_honour | POST | /v1/query_singer_honour_detail | @http://h5activity.kugou.com |  |  |
| `artist.lists` | artist_lists | GET | /ocean/v6/singer/list |  |  |  |
| `artist.unfollow` | artist_unfollow | POST | /followservice/v3/unfollow_singer |  | yes |  |
| `artist.videos` | artist_videos | GET | /kmr/v1/author/videos | @https://openapicdn.kugou.com |  |  |
| `audio` | audio | POST | /v1/audio/audio | @http://kmr.service.kugou.com router:kmr.service.kugou.com |  |  |
| `audio.accompany.matching` | audio_accompany_matching | GET | /sing7/accompanywan/json/v2/cdn/optimal_matching_accompany_2_listen.do | @https://nsongacsing.kugou.com |  |  |
| `audio.ktv.total` | audio_ktv_total | GET | /sing7/listenguide/json/v2/cdn/listenguide/get_total_opus_num_v02.do | @https://acsing.service.kugou.com |  |  |
| `audio.match` | audio_match | POST | /fingerprint.service/v1/music_trackid_mulit |  |  |  |
| `audio.related` | audio_related | GET | /v3/album_audio/related | @https://listkmrp3cdnretry.kugou.com |  |  |
| `blacklist` | blacklist | POST | /v1/edit_list_items | @https://relationuser.kugou.com | yes |  |
| `blacklist.list` | blacklist_list | POST | /v2/get_list_items | @https://relationuser.kugou.com |  |  |
| `brush` | brush | POST | /genesisapi/v1/newepoch_song_rec/feed |  |  |  |
| `captcha.sent` | captcha_sent | POST | /v7/send_mobile_code | @http://login.user.kugou.com |  |  |
| `comment.album` | comment_album | POST | /m.comment.service/v1/cmtlist |  |  |  |
| `comment.album.send` | comment_album_send | POST | /index.php |  | yes |  |
| `comment.count` | comment_count | GET | /index.php | router:sum.comment.service.kugou.com |  |  |
| `comment.floor` | comment_floor | POST | /mcomment/v1/hot_replylist |  |  |  |
| `comment.floor.send` | comment_floor_send | POST | /index.php |  | yes |  |
| `comment.music` | comment_music | POST | /mcomment/v1/cmtlist |  |  |  |
| `comment.music.classify` | comment_music_classify | POST | /mcomment/v1/cmt_classify_list |  |  |  |
| `comment.music.del` | comment_music_del | GET | /index.php |  | yes |  |
| `comment.music.hotword` | comment_music_hotword | POST | /mcomment/v1/get_hot_word |  |  |  |
| `comment.music.send` | comment_music_send | POST | /index.php |  | yes |  |
| `comment.playlist` | comment_playlist | POST | /m.comment.service/v1/cmtlist |  |  |  |
| `comment.playlist.send` | comment_playlist_send | POST | /index.php |  | yes |  |
| `effects.artist` | effects_artist | GET | /api/v3/sound/list | @http://mobilecdngz.kugou.com |  |  |
| `effects.brand` | effects_brand | GET | /api/v5/earphone/get_brand | @http://mobilecdngz.kugou.com |  |  |
| `effects.brand.detail` | effects_brand_detail | GET | /api/v5/earphone/get_model | @http://mobilecdngz.kugou.com |  |  |
| `effects.car.brand` | effects_car_brand | GET | /api/v5/car_sound/get_brand | @http://mobilecdngz.kugou.com |  |  |
| `effects.car.brand.lists` | effects_car_brand_lists | GET | /api/v3/sound/list | @http://mobilecdngz.kugou.com |  |  |
| `effects.match` | effects_match | GET | /api/v5/earphone/match | @http://mobilecdngz.kugou.com |  |  |
| `everyday.friend` | everyday_friend | POST | /sing7/relation/json/v3/friend_rec_by_using_song_list | @https://acsing.service.kugou.com |  |  |
| `everyday.history` | everyday_history | POST | /everyday/api/v1/get_history | router:everydayrec.service.kugou.com |  |  |
| `everyday.recommend` | everyday_recommend | POST | /everyday_song_recommend | router:everydayrec.service.kugou.com |  |  |
| `everyday.style.recommend` | everyday_style_recommend | POST | /everydayrec.service/everyday_style_recommend |  |  |  |
| `favorite.count` | favorite_count | GET | /count/v1/audio/mget_collect |  |  |  |
| `fm.class` | fm_class | POST | /v1/class_fm_song | router:fm.service.kugou.com |  |  |
| `fm.image` | fm_image | POST | /v1/fm_info | router:fm.service.kugou.com |  |  |
| `fm.recommend` | fm_recommend | POST | /v1/rcmd_list | router:fm.service.kugou.com |  |  |
| `fm.songs` | fm_songs | POST | /v1/fm_info | router:fm.service.kugou.com |  |  |
| `get.mode.info` | get_mode_info | GET | /api/v5/earphone/get_model_info | @http://mobileservice.kugou.com |  |  |
| `get.model` | get_model | GET | /ocean/v6/sound/list |  |  |  |
| `get.verify.info` | get_verify_info | POST | /verifyservice/v3/get_verify_info |  |  |  |
| `home.discover` | home_discover | POST | /homediscoverrec/v1/client/home_discover_rec |  |  |  |
| `images` | images | GET | /container/v2/image | @https://expendablekmr.kugou.com |  |  |
| `images.audio` | images_audio | GET | /v2/author_image/audio | @https://expendablekmr.kugou.com |  |  |
| `import.playlist` | import_playlist | POST | /assetservice/import/v1/add_task |  |  |  |
| `ip` | ip | POST | /openapi/v1/ip/audios |  |  |  |
| `ip.dateil` | ip_dateil | POST | /openapi/v1/ip |  |  |  |
| `ip.playlist` | ip_playlist | POST | /ocean/v6/pubsongs/list_info_for_ip |  |  |  |
| `ip.zone` | ip_zone | GET | /v1/zone/index | router:yuekucategory.kugou.com |  |  |
| `ip.zone.home` | ip_zone_home | GET | /v1/zone/home | router:yuekucategory.kugou.com |  |  |
| `kmr.audio.mv` | kmr_audio_mv | POST | /kmr/v1/audio/mv | router:openapi.kugou.com |  |  |
| `krm.audio` | krm_audio | POST | /kmr/v2/audio | router:openapi.kugou.com |  |  |
| `lastest.songs.listen` | lastest_songs_listen | POST | /playque/devque/v1/get_latest_songs |  |  |  |
| `listen.together.chat.history` | listen_together_chat | POST | /rmservice/v1/group/msg_history |  | yes |  |
| `listen.together.chat.send` | listen_together_chat | POST | /rmservice/v1/group/chat |  |  |  |
| `listen.together.discovery.channel_search` | listen_together_discovery | GET | /v1/search/channel | @https://youth.kugou.com |  |  |
| `listen.together.discovery.genting_recommend` | listen_together_discovery | POST | /v1/genting/recommend | @https://concepts.kugou.com |  |  |
| `listen.together.discovery.genting_square` | listen_together_discovery | GET | /v1/genting/square | @https://youth.kugou.com |  |  |
| `listen.together.discovery.genting_streamers` | listen_together_discovery | GET | /v1/genting/get_streamer_list | @https://youth.kugou.com |  |  |
| `listen.together.discovery.kugroup_square` | listen_together_discovery | GET | /v1/kugroup/square | @https://youth.kugou.com |  |  |
| `listen.together.discovery.kugroup_streamers` | listen_together_discovery | GET | /v1/kugroup/get_streamer_list | @https://youth.kugou.com |  |  |
| `listen.together.discovery.privilege` | listen_together_discovery | GET | /v1/privilege/operate | @https://youth.kugou.com |  |  |
| `listen.together.discovery.recent_rooms` | listen_together_discovery | GET | /v3/user/recent_room_dynamic | @https://youth.kugou.com |  |  |
| `listen.together.music.detail` | listen_together_music | GET | /youth/v1/genting/get_musicroom_info |  |  |  |
| `listen.together.music.history` | listen_together_music | GET | /youth/v1/genting/history |  |  |  |
| `listen.together.music.initialize` | listen_together_music | POST | /youth/v1/genting/init_musicroom |  |  |  |
| `listen.together.music.list` | listen_together_music | POST | /youth/v2/genting/recommend |  |  |  |
| `listen.together.music.members` | listen_together_music | GET | /youth/v1/genting/get_musicroom_member |  |  |  |
| `listen.together.music.music_add` | listen_together_music | POST | /youth/v1/genting/music_add |  |  |  |
| `listen.together.music.order_song` | listen_together_music | POST | /youth/v1/genting/order_song |  |  |  |
| `listen.together.music.playback_url` | listen_together_music | POST | /youth/v1/genting/music_reqcmd |  |  |  |
| `listen.together.music.player_operation` | listen_together_music | POST | /youth/v1/genting/music_player_opr |  |  |  |
| `listen.together.music.playlist` | listen_together_music | POST | /youth/v1/genting/music_fetch_list |  |  |  |
| `listen.together.music.recent_playlist` | listen_together_music | POST | /youth/v1/genting/music_recent_list |  |  |  |
| `listen.together.music.remove_song` | listen_together_music | POST | /youth/v1/genting/remove_song |  |  |  |
| `listen.together.music.song_order_list` | listen_together_music | GET | /youth/v1/genting/song_order_list |  |  |  |
| `listen.together.music.switch_song` | listen_together_music | POST | /youth/v1/genting/music_sw |  |  |  |
| `listen.together.music.sync_player` | listen_together_music | POST | /youth/v1/genting/music_sync_player |  |  |  |
| `listen.together.room.check_minor` | listen_together_room | GET | /v1/risk/check_minor | @https://youth.kugou.com |  |  |
| `listen.together.room.create` | listen_together_room | POST | /rmservice/v1/group/create |  | yes |  |
| `listen.together.room.dismiss` | listen_together_room | POST | /rmservice/v1/group/dismiss |  | yes |  |
| `listen.together.room.heartbeat` | listen_together_room | POST | /rmservice/v1/group/heartbeat |  | yes |  |
| `listen.together.room.join` | listen_together_room | POST | /rmservice/v1/group/join |  | yes |  |
| `listen.together.room.leave` | listen_together_room | POST | /rmservice/v1/group/leave |  | yes |  |
| `listen.together.room.state` | listen_together_room | POST | /rmservice/v1/group/info |  | yes |  |
| `listen.together.room.status` | listen_together_room | POST | /rmservice/v1/user/get_status |  | yes |  |
| `listen.together.room.update_chat` | listen_together_room | POST | /rmservice/v1/group/update_info |  | yes |  |
| `listen.together.study.configure` | listen_together_study | POST | /v1/user/make_room | @https://youth.kugou.com | yes |  |
| `listen.together.study.created_rooms` | listen_together_study | GET | /v1/study/user_create_room_list | @https://youth.kugou.com |  |  |
| `listen.together.study.delete_created_room` | listen_together_study | DELETE | /youth/v1/room/delete_room |  |  |  |
| `listen.together.study.detail` | listen_together_study | GET | /v1/room/get_room_detail | @https://youth.kugou.com |  |  |
| `listen.together.study.list` | listen_together_study | GET | /v1/room/get_room_list_by_tag | @https://youth.kugou.com |  |  |
| `listen.together.study.members` | listen_together_study | GET | /v1/room/get_member_list | @https://youth.kugou.com |  |  |
| `listen.together.study.playlist` | listen_together_study | POST | /v1/music/fetch_list | @https://youth.kugou.com | yes |  |
| `listen.together.study.sync_player` | listen_together_study | POST | /v1/music/sync_player | @https://youth.kugou.com | yes |  |
| `login` | login | POST | /v9/login_by_pwd | router:login.user.kugou.com |  |  |
| `login.cellphone` | login_cellphone | POST | /v7/login_by_verifycode | @https://loginserviceretry.kugou.com |  |  |
| `login.device` | login_device | POST | /v2/get_dev | @https://userinfoservice.kugou.com | yes |  |
| `login.device.kick` | login_device_kick | GET | /loginservice/v1/dev_logout |  |  |  |
| `login.openplat` | login_openplat | POST | /v6/login_by_openplat | router:login.user.kugou.com |  |  |
| `login.qq` | login_qq | POST | /v6/login_by_openplat | router:login.user.kugou.com |  |  |
| `login.qq.qr.check` | login_qq_qr_check | GET | / |  |  |  |
| `login.qq.qr.create` | login_qq_qr_create | GET | / |  |  |  |
| `login.qr.authorize` | login_qr_authorize | POST | /v2/scan | @https://login-user.kugou.com | yes |  |
| `login.qr.check` | login_qr_check | GET | /v2/get_userinfo_qrcode | @https://login-user.kugou.com |  |  |
| `login.qr.create` | login_qr_create | GET | / |  |  |  |
| `login.qr.key` | login_qr_key | GET | /v2/qrcode | @https://login-user.kugou.com |  |  |
| `login.token` | login_token | POST | /v5/login_by_token | @http://login.user.kugou.com | yes |  |
| `login.wx.check` | login_wx_check | GET | / |  |  |  |
| `login.wx.create` | login_wx_create | GET | / |  |  |  |
| `longaudio.album.audios` | longaudio_album_audios | POST | /longaudio/v2/album_audios | router:openapi.kugou.com |  |  |
| `longaudio.album.detail` | longaudio_album_detail | POST | /openapi/v2/broadcast |  |  |  |
| `longaudio.album.list` | longaudio_album_list | GET | /longaudio/v1/album/list |  |  |  |
| `longaudio.daily.recommend` | longaudio_daily_recommend | POST | /longaudio/v1/home_new/daily_recommend |  |  |  |
| `longaudio.rank.recommend` | longaudio_rank_recommend | GET | /longaudio/v1/home_new/rank_card_recommend |  |  |  |
| `longaudio.search` | longaudio_search | GET | /complexsearch/v4/search/song |  |  |  |
| `longaudio.tag.list` | longaudio_tag_list | GET | /v3/list_audiobook_tags | router:longaudio.kugou.com |  |  |
| `longaudio.vip.recommend` | longaudio_vip_recommend | POST | /longaudio/v1/home_new/vip_select_recommend |  |  |  |
| `longaudio.week.recommend` | longaudio_week_recommend | POST | /longaudio/v1/home_new/week_new_albums_recommend |  |  |  |
| `lyric` | lyric | GET | /download | @https://lyrics.kugou.com |  |  |
| `mv.collect` | mv_collect | POST | /v1/collect | @https://collectservice.kugou.com | yes |  |
| `mv.collect.del` | mv_collect_del | POST | /v1/cancel_collect | @https://collectservice.kugou.com | yes |  |
| `pc.diantai` | pc_diantai | POST | /v3/pc_diantai | @https://adservice.kugou.com |  |  |
| `personal.fm` | personal_fm | POST | /v2/personal_recommend | router:persnfm.service.kugou.com |  |  |
| `playhistory.upload` | playhistory_upload | POST | /playhistory/v1/upload_songs |  |  |  |
| `playlist.add` | playlist_add | POST | /cloudlist.service/v5/add_list |  | yes |  |
| `playlist.del` | playlist_del | POST | /v2/delete_list | router:cloudlist.service.kugou.com | yes |  |
| `playlist.detail` | playlist_detail | POST | /v3/get_list_info | router:pubsongs.kugou.com |  |  |
| `playlist.effect` | playlist_effect | POST | /pubsongs/v1/get_sound_effect_list |  |  |  |
| `playlist.pic` | playlist_pic | POST | /v1/modify_list_pic |  | yes |  |
| `playlist.pic.upload` | playlist_pic_upload | POST | /stream.php | @http://imgphp.kugou.com/imageupload |  |  |
| `playlist.share.official` | playlist_share_official | GET | /app | @http://t.service.kugou.com |  |  |
| `playlist.similar` | playlist_similar | POST | /pubsongs/v1/kmr_get_similar_lists |  |  |  |
| `playlist.sort` | playlist_sort | POST | /v1/modify_list_sort |  | yes |  |
| `playlist.tags` | playlist_tags | POST | /pubsongs/v1/get_tags_by_type |  |  |  |
| `playlist.track.all` | playlist_track_all | GET | /pubsongs/v2/get_other_list_file_nofilt |  |  |  |
| `playlist.track.all.new` | playlist_track_all_new | POST | /v4/get_list_all_file_v3 | router:cloudlist.service.kugou.com |  |  |
| `playlist.tracks.add` | playlist_tracks_add | POST | /cloudlist.service/v6/add_song |  | yes |  |
| `playlist.tracks.del` | playlist_tracks_del | POST | /v4/delete_songs | router:cloudlist.service.kugou.com |  |  |
| `playlist.tracks.sort` | playlist_tracks_sort | POST | /v1/modify_song_sort |  | yes |  |
| `playlist.update` | playlist_update | POST | /v1/modify_list |  | yes |  |
| `privilege.lite` | privilege_lite | POST | /v2/get_res_privilege/lite | router:media.store.kugou.com |  |  |
| `rank.audio` | rank_audio | POST | /openapi/kmr/v2/rank/audio |  |  |  |
| `rank.info` | rank_info | GET | /ocean/v6/rank/info |  |  |  |
| `rank.list` | rank_list | GET | /ocean/v6/rank/list |  |  |  |
| `rank.top` | rank_top | GET | /mobileservice/api/v5/rank/rec_rank_list |  |  |  |
| `rank.vol` | rank_vol | GET | /ocean/v6/rank/vol |  |  |  |
| `recommend.songs` | recommend_songs | POST | /everyday_song_recommend | router:everydayrec.service.kugou.com |  |  |
| `register.dev` | register_dev | POST | /risk/v3/r_register_dev | @https://userservice.kugou.com |  |  |
| `scene.audio.list` | scene_audio_list | POST | /scene/v1/scene/audio_list |  |  |  |
| `scene.collection.list` | scene_collection_list | POST | /scene/v1/distribution/collection_list |  |  |  |
| `scene.lists` | scene_lists | GET | /scene/v1/scene/list |  |  |  |
| `scene.lists.v2` | scene_lists_v2 | POST | /scene/v1/scene/list_v2 |  |  |  |
| `scene.module` | scene_module | POST | /scene/v1/scene/module |  |  |  |
| `scene.module.info` | scene_module_info | GET | /scene/v1/scene/module_info |  |  |  |
| `scene.music` | scene_music | POST | /genesisapi/v1/scene_music/rec_music |  |  |  |
| `scene.video.list` | scene_video_list | POST | /scene/v1/distribution/video_list |  |  |  |
| `search` | search | GET | /v2/search/song | router:complexsearch.kugou.com |  |  |
| `search.complex` | search_complex | GET | /v6/search/complex | @https://complexsearch.kugou.com |  |  |
| `search.default` | search_default | POST | /searchnofocus/v1/search_no_focus_word |  |  |  |
| `search.hot` | search_hot | GET | /api/v3/search/hot_tab |  |  |  |
| `search.lyric` | search_lyric | GET | /v1/search | @https://lyrics.kugou.com |  |  |
| `search.mixed` | search_mixed | GET | /v3/search/mixed |  |  |  |
| `search.suggest` | search_suggest | GET | /v2/getSearchTip |  |  |  |
| `server.now` | server_now | POST | /v1/server_now | router:usercenter.kugou.com |  |  |
| `sheet.collection` | sheet_collection | GET | /miniyueku/v1/opern_square/get_home_module_config |  |  |  |
| `sheet.detail` | sheet_detail | GET | /opern/v1/detail/info |  |  |  |
| `sheet.explore` | sheet_explore | POST | /opern/v1/home/get_rec_opern |  |  |  |
| `sheet.rank` | sheet_rank | POST | /opern/v1/home/get_rank_opern |  |  |  |
| `sheet.song` | sheet_song | GET | /opern/v1/detail/song_info |  |  |  |
| `sheet.tags` | sheet_tags | GET | /opern/v1/home/get_tags |  |  |  |
| `sidedt` | sidedt | POST | /v4/verify_user_info | @https://verifyservice.kugou.com | yes |  |
| `singer.list` | singer_list | GET | /ocean/v6/singer/list |  |  |  |
| `song.auth` | song_auth | GET | /v1/authorization | router:trackercdngz.kugou.com | yes |  |
| `song.barrage` | song_barrage | GET | /index.php |  |  |  |
| `song.barrage.send` | song_barrage_send | POST | /index.php |  | yes |  |
| `song.climax` | song_climax | GET | /v1/audio_climax/audio | @https://expendablekmrcdn.kugou.com |  |  |
| `song.ranking` | song_ranking | GET | /grow/v1/song_ranking/play_page/ranking_info |  |  |  |
| `song.ranking.filter` | song_ranking_filter | GET | /grow/v1/song_ranking/unlock/v2/ranking_filter |  |  |  |
| `song.url` | song_url | GET | /v5/url | router:trackercdn.kugou.com |  |  |
| `song.url.auth` | song_url_auth | GET | /tracker/v5/url |  | yes |  |
| `song.url.auth.merge` | song_url_auth_merge | GET | /v1/authorization | router:trackercdngz.kugou.com | yes |  |
| `song.url.new` | song_url_new | POST | /v6/priv_url | router:tracker.kugou.com | yes |  |
| `team.history` | team_history | GET | /youth/v1/ut/get_team_record |  | yes |  |
| `team.join` | team_join | POST | /youth/v1/ut/join_team |  |  |  |
| `team.my` | team_my | POST | /youth/v1/ut/create_team |  |  |  |
| `team.my.info` | team_my_info | GET | /youth/v1/ut/get_my_team_info |  | yes |  |
| `team.my.status` | team_my_status | GET | /youth/v1/ut/get_user_status |  | yes |  |
| `team.period.info` | team_period_info | GET | /youth/v1/ut/get_period_info |  | yes |  |
| `theme.music` | theme_music | POST | /everydayrec.service/v1/mul_theme_category_recommend |  |  |  |
| `theme.music.detail` | theme_music_detail | POST | /everydayrec.service/v1/theme_category_recommend |  |  |  |
| `theme.playlist` | theme_playlist | POST | /v2/getthemelist | router:everydayrec.service.kugou.com |  |  |
| `theme.playlist.track` | theme_playlist_track | POST | /v2/gettheme_songidlist | router:everydayrec.service.kugou.com |  |  |
| `top.album` | top_album | POST | /musicadservice/v1/mobile_newalbum_sp |  |  |  |
| `top.card` | top_card | POST | /singlecardrec.service/v1/single_card_recommend |  |  |  |
| `top.card.youth` | top_card_youth | POST | /youth/v1/song/single_card_recommend |  |  |  |
| `top.ip` | top_ip | POST | /v1/daily_recommend | @http://musicadservice.kugou.com |  |  |
| `top.playlist` | top_playlist | POST | /v2/special_recommend | router:specialrec.service.kugou.com |  |  |
| `top.song` | top_song | POST | /musicadservice/container/v1/newsong_publish |  |  |  |
| `user.batch.union.vipinfo` | user_batch_union_vipinfo | POST | /v2/batch_union_vipinfo | @https://kugouvip.kugou.com | yes |  |
| `user.cloud` | user_cloud | POST | /v1/get_list | @https://mcloudservice.kugou.com | yes |  |
| `user.cloud.del` | user_cloud_del | POST | /v1/del_files | @https://mcloudservice.kugou.com | yes |  |
| `user.cloud.match` | user_cloud_match | POST | /v2/album_audio/audio | @http://kmr.service.kugou.com router:kmr.service.kugou.com |  |  |
| `user.cloud.upload` | user_cloud_upload | POST | /v1/add_files | @https://mcloudservice.kugou.com | yes |  |
| `user.cloud.url` | user_cloud_url | GET | /bsstrackercdngz/v2/query_musicclound_url |  | yes |  |
| `user.detail` | user_detail | POST | /v3/get_my_info | router:usercenter.kugou.com |  |  |
| `user.fans` | user_fans | POST | /v2/fans_list | @https://relationuser.kugou.com |  |  |
| `user.follow` | user_follow | POST | /v4/follow_list | router:relationuser.kugou.com |  |  |
| `user.follow.add` | user_follow_add | POST | /v2/follow | router:relation.user.kugou.com |  |  |
| `user.follow.chat` | user_follow_chat | POST | /v1/chat/send | router:msg.mobile.kugou.com | yes |  |
| `user.follow.del` | user_follow_del | POST | /v1/unfollow | router:relationuser.kugou.com |  |  |
| `user.follow.message` | user_follow_message | GET | /msg.mobile/v3/msgtag/history |  |  |  |
| `user.friends` | user_friends | POST | /v1/friends_list | @https://relationuser.kugou.com |  |  |
| `user.grade.info` | user_grade_info | POST | /v2/get_grade_info | @http://userinfo.user.kugou.com | yes |  |
| `user.history` | user_history | POST | /playhistory/v1/get_songs |  |  |  |
| `user.info` | user_info | POST | /v1/get_my_userinfo | @http://relation.user.kugou.com |  |  |
| `user.listen` | user_listen | POST | /v2/get_list | @https://listenservice.kugou.com |  |  |
| `user.listen.report` | user_listen_report | POST | /v2/post | @http://d.kugou.com | yes |  |
| `user.playlist` | user_playlist | POST | /v7/get_all_list | router:cloudlist.service.kugou.com |  |  |
| `user.preference` | user_preference | POST | /userpreferservice/v1/get_user_conf |  | yes |  |
| `user.preference.update` | user_preference_update | POST | /userpreferservice/v1/update_user_conf |  | yes |  |
| `user.purchased.albums` | user_purchased_albums | POST | /openapi/v1/copyright/get_album_goods |  |  |  |
| `user.purchased.songs` | user_purchased_songs | POST | /openapi/copyright/v1/audio/get_goods |  |  |  |
| `user.update` | user_update | POST | /v1/update_userinfo | @http://update.user.kugou.com |  |  |
| `user.update.avatar` | user_update_avatar | POST | /v1/profile |  | yes |  |
| `user.verify` | user_verify | GET | /v1/user_verify | router:trackercdngz.kugou.com |  |  |
| `user.video.collect` | user_video_collect | POST | /collectservice/v2/collect_list_mixvideo |  |  |  |
| `user.video.love` | user_video_love | GET | /m.comment.service/v1/get_user_like_video |  |  |  |
| `user.vip.detail` | user_vip_detail | GET | /v1/get_union_vip | @https://kugouvip.kugou.com |  |  |
| `user.visitors` | user_visitors | POST | /v2/get_visitors | @http://usercenter.kugou.com | yes |  |
| `verify.user.info` | verify_user_info | POST | /v4/verify_user_info | @https://verifyservice.kugou.com |  |  |
| `video.barrage` | video_barrage | GET | /index.php |  |  |  |
| `video.barrage.send` | video_barrage_send | POST | /index.php |  | yes |  |
| `video.detail` | video_detail | POST | /v1/video | router:kmr.service.kugou.com |  |  |
| `video.privilege` | video_privilege | POST | /v1/get_video_privilege | router:media.store.kugou.com |  |  |
| `video.url` | video_url | GET | /v2/interface/index | router:trackermv.kugou.com |  |  |
| `youth.channel.all` | youth_channel_all | GET | /youth/v2/channel/channel_all_list |  |  |  |
| `youth.channel.amway` | youth_channel_amway | GET | /youth/api/amway/v2/index |  |  |  |
| `youth.channel.detail` | youth_channel_detail | POST | /youth/api/channel/v1/channel_list_by_id |  |  |  |
| `youth.channel.similar` | youth_channel_similar | POST | /youth/v1/channel/get_friendly_channel |  |  |  |
| `youth.channel.song` | youth_channel_song | GET | /youth/api/channel/v1/channel_get_song_audit_passed |  |  |  |
| `youth.channel.song.detail` | youth_channel_song_detail | GET | /youth/v2/post/get_song_detail |  |  |  |
| `youth.channel.song.list` | youth_channel_song_list | GET | /api/channel/v1/channel_get_song_list | @https://youth.kugou.com |  |  |
| `youth.channel.sub` | youth_channel_sub | DELETE | /youth/v1/channel_un_subscribe |  |  |  |
| `youth.day.vip` | youth_day_vip | POST | /youth/v1/recharge/receive_vip_listen_song |  |  |  |
| `youth.day.vip.upgrade` | youth_day_vip_upgrade | POST | /youth/v1/listen_song/upgrade_vip_reward |  |  |  |
| `youth.dynamic` | youth_dynamic | GET | /youth/v3/user/get_dynamic |  |  |  |
| `youth.dynamic.recent` | youth_dynamic_recent | GET | /youth/v3/user/recent_dynamic |  |  |  |
| `youth.listen.song` | youth_listen_song | POST | /youth/v2/report/listen_song |  |  |  |
| `youth.month.vip.record` | youth_month_vip_record | GET | /youth/v1/activity/get_month_vip_record |  |  |  |
| `youth.union.vip` | youth_union_vip | GET | /v1/get_union_vip | @https://kugouvip.kugou.com |  |  |
| `youth.user.song` | youth_user_song | GET | /youth/v1/get_user_song_public |  |  |  |
| `youth.vip` | youth_vip | POST | /youth/v1/ad/play_report |  |  |  |
| `yueku` | yueku | GET | /v1/yueku/recommend_v2 | router:service.mobile.kugou.com |  |  |
| `yueku.banner` | yueku_banner | POST | /ads.gateway/v3/listen_banner |  |  |  |
| `yueku.fm` | yueku_fm | GET | /v1/time_fm_info | router:fm.service.kugou.com |  |  |
