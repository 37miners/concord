// concord's own js functions

var cur_server = '';
var cur_channel = '';
var cur_pubkey = '';
var servers = '';
var message_count = 0;

var stopEnabled = false;
var iconId = '';
var curName = '';
var listener_id_global =
	String(Math.floor(Math.random() * 9007199254740991)) +
	String(Math.floor(Math.random() * 9007199254740991));
var menu = [{
            name: 'Invite',
            img: 'images/create.png',
            fun: function () {
		show_create_invite();
            }
        }, {
            name: 'Configure',
            img: 'images/update.png',
            fun: function () {
		modify_server(iconId, curName);
            }
        }, {
            name: 'Delete',
            img: 'images/delete.png',
            fun: function () {
                var req = new XMLHttpRequest();
                req.addEventListener("load", function() {
                        load_server_bar();
                });
                req.open("GET", '/delete_server?server_id='+iconId);
                req.send();
            }
}];

function create_invite() {
	var num = Number(document.forms["invite"]["max_accepts"].value);

	if (isNaN(num) || num < 1) {
		alert('max accepts must be a positive number');
	} else {
		var expiry = 0;

		if (document.forms["invite"]["expiration"].value == "onehour") {
			expiry = Date.now() + 1000 * 60 * 60;
		} else if (document.forms["invite"]["expiration"].value == "oneday") {
			expiry = Date.now() + 1000 * 60 * 60 * 24;
		} else if (document.forms["invite"]["expiration"].value == "oneweek") {
			expiry = Date.now() + 1000 * 60 * 60 * 24 * 7;
		} else { // forever
			expiry = 0;
		}
                var req = new XMLHttpRequest();
                req.addEventListener("load", function() {
			load_invite_list();
                });
                req.open("GET", '/create_invite?server_id='+iconId+'&count='+num+'&expiry='+expiry);
                req.send();
	}
}

function makeid(length) {
	var result           = '';
	var characters       = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
	var charactersLength = characters.length;
	for ( var i = 0; i < length; i++ ) {
		result += characters.charAt(Math.floor(Math.random() * 
		charactersLength));
	}
	return result;
}

function close_interstitial() {
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
	document.getElementById('interstitial').style.visibility = 'hidden';
}

function show_auth_error() {
	document.getElementById('interstitial').style.visibility = 'visible';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
}

function show_create_invite() {
        document.getElementById('interstitial').style.visibility = 'visible';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding7").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
	load_invite_list();
}

function create_server() {
	document.getElementById("interstitialtextpadding2").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
}

function join_server() {
        document.getElementById("interstitialtextpadding3").style.visibility = 'visible';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
}

function modify_server(iconId, curName) {
	var rand = makeid(8);
	document.getElementById('curImage').src = '/get_server_icon?server_id=' + iconId + '&r=' + rand;
	document.forms['modify']['id'].value = iconId;
	document.forms['modify']['name'].value = curName;
	document.getElementById('interstitial').style.visibility = 'visible';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
}

function show_invite_info() {
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding8").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
}

function show_profile_settings(pubkey) {
	document.forms['avatar']['pubkey'].value = pubkey;
	document.getElementById('interstitial').style.visibility = 'visible';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'visible';
}

function join_server_link() {
	var link = document.forms['join']['link'].value;

	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		var invite_info = JSON.parse(this.responseText);
		document.getElementById('inviteserveruser').innerHTML = invite_info.inviter_pubkey;
		document.getElementById('inviteservername').innerHTML = invite_info.name;
		var rand = makeid(8);
		document.getElementById('inviteservericon').src =
			'/get_server_icon?server_id=' + invite_info.server_id +
			'&server_pubkey='+invite_info.server_pubkey +
			'&r=' + rand;
		document.forms['viewinvite']['link'].value = link;
		show_invite_info();
	});
	req.open("GET", '/view_invite?link=' + encodeURIComponent(link));
	req.send();
}

function load_invite_list() {
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		var invite_div = document.getElementById('invite_div');
		invite_div.innerHTML = '';
		var invites = JSON.parse(this.responseText);
		invites.forEach(function(invite) {
			invite_div.appendChild(document.createTextNode(invite.url));
			var delete_button = document.createElement('img');
			delete_button.src = '/images/delete.png';
			delete_button.className = 'delete_invite noselect';
			delete_button.onclick = function() {
				var req = new XMLHttpRequest();
				req.addEventListener("load", function() {

					load_invite_list();
				});
				req.open("GET", '/revoke_invite?invite_id='+invite.id);
				req.send();
			};
			invite_div.appendChild(delete_button);
			invite_div.appendChild(document.createElement('br'));
		});
	
	});
	req.open("GET", '/list_invites?server_id=' + iconId);
	req.send();
}

function modify_server_submit() {
        var form = document.getElementById('modify');
        var icon = form.file.files[0];
	var iconId = document.forms['modify']['id'].value;
        if (icon) {
                var name = form.name.value;

                var req = new XMLHttpRequest();
                var formData = new FormData();
                formData.append("icon", icon);
                req.addEventListener("load", function() {
                        load_server_bar();
                        close_interstitial();
                });
                req.open("POST", '/modify_server?server_id=' + iconId + '&name='+encodeURIComponent(name));
                req.send(formData);
        } else {
                alert("ERROR: file must be specified.");
        }
}

function create_channel() {
	var server_id = document.forms['channel']['server_id'].value;
	var name = document.forms['channel']['name'].value;
	var description = document.forms['channel']['description'].value;

	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		close_interstitial();
	});
	req.open(
		"GET",
		'/set_channel?name='+encodeURIComponent(name)+
		'&description='+encodeURIComponent(description)+
		'&server_id='+server_id
	);
	req.send();
}

function do_add_channel(server_id) {
	document.forms['channel']['server_id'].value = server_id;
        document.getElementById('interstitial').style.visibility = 'visible';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
}

function subscribe(server, listener_id) {
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {});
	req.open("GET", '/subscribe?server_id='+server.id+'&server_pubkey='+server_pubkey+'&listener_id='+listener_id);
	req.send();
}

function add_message_to_chat_area(
	chat_area,
	server_id,
	server_pubkey,
	user_pubkey_urlencoded,
	user_pubkey_onion,
	message_text,
	timestamp,
	user_name,
	user_bio,
	odd
) {
	var div = document.createElement('div');
	if (odd) {
		div.className = 'message_div_odd noselect';
	} else {
		div.className = 'message_div_even noselect';
	}
	var img = document.createElement('img');
	var rand = makeid(8);
	img.src = '/get_profile_image?server_id=' + server_id +
		'&server_pubkey=' + server_pubkey +
		'&user_pubkey=' + user_pubkey_urlencoded +
		'&rand=' + rand;
	img.title = user_bio;
	img.className = 'mini_profile_avatar noselect';
	div.appendChild(img);
	var date = new Date();
	date.setTime(timestamp);
	var user_name_span = document.createElement('span');
	user_name_span.className = 'message_user_name_span noselect';
	user_name_span.appendChild(document.createTextNode(user_name + '> '));
	var m = document.createTextNode(
		message_text
	);
	var text_span = document.createElement('span');
	text_span.className = 'message_text_span noselect';
	text_span.appendChild(user_name_span);
	text_span.appendChild(m);

	var date_span = document.createElement('span');
	date_span.className = 'message_date_span noselect';
	var d = document.createTextNode(
		date.toLocaleString()
	);
	date_span.appendChild(d);
	div.appendChild(date_span);

	div.appendChild(text_span);
	chat_area.appendChild(div);
	var hr = document.createElement('hr');
	hr.className = 'message_hr noselect';
	chat_area.appendChild(hr);
}

function load_server_bar() {
	// first clear server bar
	document.getElementById('serverbartext').innerHTML = '';

	// then load all servers
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		var load_listener;
		if (servers == '') {
			load_listener = true;
		} else {
			load_listener = false;
		}
		servers = JSON.parse(this.responseText);

		if (servers.error !== undefined) {
			// authentication error
			show_auth_error();
		} else {
			if (load_listener) listener();
			servers.forEach(function(server) {
				var server_pubkey = server.server_pubkey;
				var serverbar = document.getElementById('serverbartext');
				var img = document.createElement('img');
				var rand = makeid(8);
				img.src = '/get_server_icon?server_id=' + server.id + '&server_pubkey='+server_pubkey + '&r=' + rand;
				img.className = 'server_icon noselect';
				var sname = decodeURIComponent(server.name);
				img.title = sname;
				img.id = server.id;
				img.onmouseover = function() {
					stopEnabled = true;
					iconId = server.id;
					curName = sname;
				};
				img.onmouseout = function() {
					stopEnabled = false;
				};
				img.onclick = function() {
					var req = new XMLHttpRequest();
					req.addEventListener("load", function() {
						load_members(server.id, server_pubkey);
						var server_name_div = document.getElementById('server_name');
						server_name_div.className = 'server_name_div noselect';
						server_name_div.innerHTML = sname;
						var add_channel = document.createElement('img');
						add_channel.className = 'add_channel_button';
						add_channel.onclick = function(evt) {
							do_add_channel(server.id);
						};
						add_channel.src = '/images/add_channel.png';
						add_channel.title = 'Add a Channel';
						server_name_div.appendChild(add_channel);
						var hr = document.createElement('hr');
						hr.className = 'channel_hr';
						server_name_div.appendChild(hr);
						var channels = JSON.parse(this.responseText);
						var channel_list = document.getElementById('channel_list');
						channel_list.innerHTML = '';
						channels.forEach(function(channel) {
							var channel_div = document.createElement('div');
							var channel_text = document.createTextNode('#' + channel.name);
							var channel_link = document.createElement('a');
							channel_link.onclick = function() {
								cur_channel = channel.id;
								cur_server = server.id;
								cur_pubkey = server.server_pubkey;
								var req = new XMLHttpRequest();
								var chat_area = document.getElementById('chat_area');
								var loading = document.createElement('img');
								loading.src = '/images/Loading_icon.gif';
								loading.className = 'loading noselect';
								chat_area.innerHTML = '';
								chat_area.appendChild(loading);

								req.addEventListener("load", function() {
									var chat_area = document.getElementById('chat_area');
									chat_area.innerHTML = '';
									try {
										var messages = JSON.parse(this.responseText);
										messages.forEach(function(message) {
											add_message_to_chat_area(
												chat_area,
												cur_server,
												cur_pubkey,
												message.user_pubkey_urlencoded,
												message.user_pubkey,
												message.text,
												message.timestamp,
												message.user_name,
												message.user_bio,
												message_count % 2 == 0,
											);
											message_count++;
										});
										chat_area.scrollTop = chat_area.scrollHeight;
									} catch(ex) {
										console.error('error='+ex+'response='+this.responseText);
									}
								});
								req.open(
									"GET",
									'/query_messages?server_id='+server.id+
									'&channel_id='+channel.id+
									'&server_pubkey='+server_pubkey
								);
								req.send();
							}
							channel_link.appendChild(channel_text);
							channel_div.appendChild(channel_link);
							channel_div.className = 'channel_div noselect';
							channel_list.appendChild(channel_div);

							var delete_button = document.createElement('div');
							delete_button.innerHTML = '<svg width="16" height="19" viewBox="0 0 16 19" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M11.681 2.41458C11.7194 2.55015 11.874 2.66 12.0286 2.66H15.0175C15.5603 2.66 16 3.05682 16 3.54666C16 3.95141 15.6996 4.29181 15.2906 4.39869L14.7774 17.1327C14.7358 18.1687 13.7807 19 12.6294 19H3.37062C2.22147 19 1.26424 18.1678 1.22258 17.1327L0.709425 4.39869C0.300438 4.29181 0 3.95141 0 3.54666C0 3.05682 0.439696 2.66 0.982467 2.66H3.97141C4.12602 2.66 4.28173 2.54817 4.31901 2.41458L4.52844 1.65557C4.78831 0.717461 5.80587 0 6.87606 0H9.12388C10.1952 0 11.2116 0.717436 11.4715 1.65557L11.681 2.41458ZM7.29835 6.71337V15.3267C7.29835 15.676 7.61305 15.96 8.00011 15.96C8.38718 15.96 8.70187 15.676 8.70187 15.3267V6.71337C8.70187 6.36405 8.38718 6.08004 8.00011 6.08004C7.61305 6.08004 7.29835 6.36405 7.29835 6.71337ZM4.07025 6.73217L4.35095 15.3455C4.36301 15.6948 4.68648 15.9699 5.07354 15.96C5.46061 15.9491 5.76542 15.6572 5.75448 15.3079L5.47377 6.69458C5.46171 6.34526 5.13824 6.07016 4.75118 6.08004C4.36412 6.09093 4.0593 6.38285 4.07025 6.73217ZM10.5265 6.69457L10.2458 15.3078C10.2348 15.6572 10.5396 15.9491 10.9267 15.96C11.3137 15.9699 11.6372 15.6948 11.6493 15.3454L11.93 6.73216C11.9409 6.38284 11.6361 6.09093 11.249 6.08003C10.862 6.07014 10.5385 6.34525 10.5265 6.69457ZM6.41553 2.66H9.58441C9.66117 2.66 9.70941 2.60558 9.69077 2.53829L9.56467 2.08505C9.5241 1.93661 9.29274 1.77332 9.12388 1.77332H6.87606C6.7072 1.77332 6.47583 1.93661 6.43527 2.08505L6.30917 2.53829C6.29053 2.60657 6.33877 2.66 6.41553 2.66H6.41553Z" fill="#EB5757"/></svg>';
							delete_button.title = 'delete channel';
							delete_button.className = 'delete_channel noselect';
							delete_button.onclick = function(evt) {
								var req = new XMLHttpRequest();
								req.addEventListener("load", function() {
							
								});
								req.open(
									"GET",
									'/delete_channel?server_id='+server.id+
									'&channel_id='+channel.id
								);
								req.send();
							};
							channel_div.appendChild(delete_button);
						});
					});
					req.open("GET", '/get_channels?server_id='+server.id+'&server_pubkey='+server_pubkey);
					req.send();
				}

				serverbar.appendChild(img);
				serverbar.appendChild(document.createElement('br'));
				$('.server_icon').contextMenu(menu, {triggerOn:'contextmenu'});
			});
		}
	});
	req.open("GET", '/get_servers');
	req.send();
}

function add_server() {
	var form = document.getElementById('create');
	var icon = form.file.files[0];
	if (icon) {
		var name = form.name.value;

		var req = new XMLHttpRequest();
		var formData = new FormData();
		formData.append("icon", icon);
		req.addEventListener("load", function() {
        		load_server_bar();
        		close_interstitial();
		});
		req.open("POST", '/create_server?name='+encodeURIComponent(name));
		req.send(formData);
	} else {
		alert("ERROR: file must be specified.");
	}
}

function init_concord() {
	var div1 = document.createElement('div');
	div1.className = 'serverbar noselect';
	document.body.appendChild(div1);
	var plusdiv = document.createElement('div');
	var plus = document.createElement('div');
	plus.className = 'plusicon noselect';
	plus.title = 'Add A Server';
	plus.alt = 'Add A Server';
	plus.onclick = function() {
		document.getElementById('interstitial').style.visibility = 'visible';
        	document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        	document.getElementById("interstitialtextpadding1").style.visibility = 'visible';
        	document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
                document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
		document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
		document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
		document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
		document.getElementById("interstitialtextpadding8").style.visibility = 'hidden';
		document.getElementById("interstitialtextpadding9").style.visibility = 'hidden';
	}

	plusdiv.appendChild(plus);
	div1.appendChild(plusdiv);
	var br = document.createElement('br');
	div1.appendChild(br);
	var serverbartext = document.createElement('div');
	serverbartext.className = 'serverbartext noselect';
	serverbartext.id = 'serverbartext';
	div1.appendChild(serverbartext);

        var div2 = document.createElement('div');
	var server_name = document.createElement('div');
	server_name.id = 'server_name';
	server_name.innerHTML = '';
	div2.appendChild(server_name);
        div2.className = 'channelbar noselect';

	var mini_profile = document.createElement('div');
	var mini_profile_loading_text = document.createElement('div');
	mini_profile_loading_text.className = 'mini_profile_loading_text noselect';
	mini_profile_loading_text.appendChild(document.createTextNode('Mini Profile Loading...'));
	mini_profile.appendChild(mini_profile_loading_text);
	mini_profile.id = 'mini_profile';
	mini_profile.className = 'mini_profile noselect';
	var mini_profile_loading = document.createElement('div');
	mini_profile_loading.className = 'loader noselect';
	mini_profile.appendChild(mini_profile_loading);
	div2.appendChild(mini_profile);

	load_mini_profile();

	var channel_list = document.createElement('div');
	channel_list.id = 'channel_list';
	channel_list.className = 'channel_list noselect';
	div2.appendChild(channel_list);
        document.body.appendChild(div2);

        var div3 = document.createElement('div');
        div3.innerHTML = '&nbsp;';
        div3.className = 'messagebar noselect';

	var input_div = document.createElement('div');
	var textarea = document.createElement('input');
	textarea.type = 'text';
	textarea.className = 'message_input noselect';

	textarea.addEventListener('keydown', function(evt) {
		if (evt.keyCode == 13) {
			var req = new XMLHttpRequest();
			var formData = new FormData();
			formData.append("payload", this.value);
			var this_ref = this;
			req.addEventListener("load", function() {
				this_ref.value = '';
			});
			var ms = Date.now();
			req.open(
				"POST",
				'/send_message?server_id='+cur_server+
				'&server_pubkey='+cur_pubkey+
				'&channel_id='+cur_channel+
				'&timestamp='+ms
			);
			req.send(formData);
		
		}
	});
	
	var chat_area = document.createElement('div');
	chat_area.innerHTML = '';
	chat_area.className = 'chat_area noselect';
	chat_area.id = 'chat_area';
	div3.appendChild(input_div);
	input_div.appendChild(textarea);
	input_div.appendChild(chat_area);
	input_div.className = 'input_div noselect';

        document.body.appendChild(div3);

        var div4 = document.createElement('div');
        div4.innerHTML = '';
        div4.className = 'statusbar noselect';
	div4.id = 'statusbar';
        document.body.appendChild(div4);

	load_server_bar();
}

function load_mini_profile() {
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		var mini_profile_resp = JSON.parse(this.responseText);
		var pubkey = mini_profile_resp[1];
		var user_name = mini_profile_resp[0].user_name;
		var user_bio = mini_profile_resp[0].user_bio;
		var img = document.createElement('img');
		var rand = makeid(8);
		img.src =
			'/get_profile_image?server_id=AAAAAAAAAAA%3D&user_pubkey=' + pubkey +
			'&server_pubkey=' + pubkey +
			'&rand=' + rand;
		img.className = 'mini_profile_avatar noselect';
		var text_div = document.createElement('div');
		text_div.innerHTML = user_name;
		text_div.className = 'mini_profile_text_div noselect';
		var gear = document.createElement('img');
		gear.src = '/images/gear.png';
		gear.className = 'gearimg noselect';

		var mini_profile =  document.getElementById('mini_profile');
		mini_profile.onclick = function(evt) {
			show_profile_settings(pubkey);
		};
		mini_profile.innerHTML = '';
		mini_profile.appendChild(img);
		mini_profile.appendChild(text_div);
		mini_profile.appendChild(gear);
		mini_profile.title = user_bio;

	});
	req.open(
		"POST",
		"/get_mini_profile"
	);
	req.send();
}

function process_response(response) {
	if (response == '') return;
        var end = response.indexOf("//-----ENDJSON-----");
	if(end>=0)
		response = response.substring(0, end);
	var events = JSON.parse(response);
	events.forEach(function(event) {
		if (event.etype == 0) {
			listener_id = event.ebody;
			servers.forEach(function(server) {
				var req = new XMLHttpRequest();
        			req.addEventListener("load", function() {
        			});
        			req.open(
					"GET",
					'/subscribe?server_id='+server.id+
					'&server_pubkey='+server.server_pubkey+
					'&listener_id='+listener_id
				);
        			req.send()
			});
			ping(listener_id, 0);
		} else if (event.etype == 1) {
			if (event.channel_id == cur_channel &&
			event.server_id == cur_server &&
			event.server_pubkey == cur_pubkey) {
				var chat_area = document.getElementById('chat_area');
				add_message_to_chat_area(
					chat_area,
					event.server_id,
					event.server_pubkey,
					event.user_pubkey_urlencoded,
					event.user_pubkey,
					event.ebody,
					event.timestamp,
					event.user_name,
					event.user_bio,
					message_count % 2 == 0,
				);
				message_count++;
				chat_area.scrollTop = chat_area.scrollHeight;
			}
		} else if (event.etype == 3) { // pong complete
		}
	});
	listener();
}

function listener() {
	var listener_id = listener_id_global;
	var req = new XMLHttpRequest();
	var start = 0;

	req.addEventListener("load", function() {
		var response = this.responseText;
		try {
			process_response(response);
		} catch(ex) {
			console.error('exception: ' + ex + ',response='+response);
		}
	});
	var rand = makeid(8);
	req.open("POST", '/listen?r='+rand+'&listener_id='+listener_id);
	
	var post_data = '';
	var i = 0;

	servers.forEach(function(server) {
		post_data +=
			'server_pubkey=' +
			server.server_pubkey +
			'&server_id=' + server.id +
			'&channel_id=0&seqno=0';
		if (i < servers.length - 1) {
			post_data += '\r\n';
		}

		i++;
	});
	req.send(post_data);
}

function ping(listener_id, i) {
	setTimeout(
		function() {
			var req = new XMLHttpRequest();
			if (i >= 2) {
				req.open("GET", '/ping?listener_id='+listener_id+'&disconnect=true');
				req.send();
			} else {
				req.open("GET", '/ping?listener_id='+listener_id);
				req.send();
				ping(listener_id, i + 1);
			}
		},
		30000
	);
}

function load_members(sname, spubkey) {
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
 		var members = JSON.parse(this.responseText);
                var status_bar = document.getElementById('statusbar');
                status_bar.innerHTML = '';
                members.forEach(function(member) {
			var member_div = document.createElement('div');
			member_div.className = 'member_div noselect';
			var rand = makeid(8);
			var img = document.createElement('img');
			img.src = '/get_profile_image?server_id=' + sname +
				'&server_pubkey=' + spubkey +
				'&user_pubkey=' + member.user_pubkey_urlencoded +
				'&rand=' + rand;
			img.title = member.user_bio;
			img.className = 'member_avatar noselect';
			member_div.appendChild(img);
			var user_name_span = document.createElement('span');
			user_name_span.className = 'user_name_span noselect';
			var user_name_str = member.user_name;
			user_name_str = member.user_name;
			if (member.user_name == "") {
				user_name_str = member.user_pubkey.substring(0, 10);
			}
			user_name_span.appendChild(
                               	document.createTextNode(
					user_name_str + " "
                               	)
			);
			if (member.user_type == 1) {
				var crown = document.createElement('img');
				crown.title = "Owner";
				crown.src = '/images/crown.png';
				crown.className = 'crown noselect';
				user_name_span.appendChild(crown);
			}
			member_div.appendChild(user_name_span);
			status_bar.appendChild(member_div);
			
		});
	});
	req.open("GET", '/get_members?server_id='+sname+'&server_pubkey=' + spubkey);
	req.send();
}

function accept_invitation() {
	var link = encodeURIComponent(document.forms['viewinvite']['link'].value);
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		let resp = JSON.parse(this.responseText);
		if (resp.success) {
			close_interstitial();
			load_server_bar();
		} else {
			alert("invite failed!");
		}
	});
	req.open("GET", "/join_server?link=" + link);
	req.send();
}

function update_profile() {
	var pubkey = document.forms['avatar']['pubkey'].value;
	var user_name = document.forms['profile']['user_name'].value;
	var user_bio = document.forms['profile']['user_bio'].value;
	var rand = makeid(8);
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		close_interstitial();
		load_mini_profile();
	});
	req.open(
		"GET", 
		'/set_profile_data?server_id=AAAAAAAAAAA%3D&server_pubkey=' + pubkey +
		'&user_pubkey=' + pubkey +
		'&user_bio=' + encodeURIComponent(user_bio) +
		'&user_name=' + encodeURIComponent(user_name) +
		'&rand=' + rand
	);
	req.send();
}

window.onunload = function(event) { 
	var req = new XMLHttpRequest();
	req.open("GET", '/disconnect?listener_id='+listener_id_global);
	req.send();
}

window.onload = function(event) {
        var fileInput = document.getElementById('avatar_input');
        fileInput.onchange = () => {
		var form = document.getElementById('avatar');
		var avatar = form.avatar_input.files[0];
		var pubkey = document.forms['avatar']['pubkey'].value;
		if (avatar) {
                	var req = new XMLHttpRequest();
                	var formData = new FormData();
                	formData.append("avatar", avatar);
                	req.addEventListener("load", function() {
                        	close_interstitial();
				load_mini_profile();
                	});
			var rand = makeid(8);
                	req.open(
				"POST",
				'/set_profile_image?server_id=AAAAAAAAAAA%3D&server_pubkey=' + pubkey +
				'&user_pubkey=' + pubkey +
				'&rand=' + rand
			);
                	req.send(formData);
        	} else {
                	alert("ERROR: file must be specified.");
        	}
	}
}

