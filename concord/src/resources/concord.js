// concord's own js functions

var cur_server = '';
var cur_channel = '';
var cur_pubkey = '';
var servers = '';

var stopEnabled = false;
var iconId = '';
var curName = '';
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
	document.getElementById('interstitial').style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
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
}

function join_server() {
        document.getElementById("interstitialtextpadding3").style.visibility = 'visible';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding7").style.visibility = 'hidden';
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
}

function join_server_link() {
	var link = document.forms['join']['link'].value;
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
	});
        req.open("GET", '/join_server?link=' + encodeURIComponent(link));
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
			delete_button.className = 'delete_invite';
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
}

function subscribe(server, listener_id) {
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {});
	req.open("GET", '/subscribe?server_id='+server.id+'&server_pubkey='+server_pubkey+'&listener_id='+listener_id);
	req.send();
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

		if (load_listener) listener();

		if (servers.error !== undefined) {
			// authentication error
			show_auth_error();
		} else {
			servers.forEach(function(server) {
				var server_pubkey = server.server_pubkey;
				var serverbar = document.getElementById('serverbartext');
				var img = document.createElement('img');
				var rand = makeid(8);
				img.src = '/get_server_icon?server_id=' + server.id + '&server_pubkey='+server_pubkey + '&r=' + rand;
				img.className = 'server_icon';
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
						load_members(server.id);
						var server_name_div = document.getElementById('server_name');
						server_name_div.innerHTML = sname;
						var add_channel = document.createElement('img');
						add_channel.onclick = function(evt) {
							do_add_channel(server.id);
						};
						add_channel.src = '/images/create.png';
						server_name_div.appendChild(add_channel);
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
								req.addEventListener("load", function() {
									var chat_area = document.getElementById('chat_area');
									chat_area.innerHTML = '';
try {
									var messages = JSON.parse(this.responseText);
									messages.forEach(function(message) {
										var m = document.createTextNode(
											message.user_pubkey.substring(0, 10) +
											'> ' +
											message.text
										);
										chat_area.appendChild(m);
										chat_area.appendChild(document.createElement('br'));
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
							channel_div.className = 'channel_div';
							channel_list.appendChild(channel_div);
							var delete_button = document.createElement('img');
							delete_button.src = '/images/delete.png';
							delete_button.className = 'delete_channel';
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
	div1.className = 'serverbar';
	document.body.appendChild(div1);
	var plusdiv = document.createElement('div');
	var plus = document.createElement('div');
	plus.className = 'plusicon';
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
	}

	plusdiv.appendChild(plus);
	div1.appendChild(plusdiv);
	var br = document.createElement('br');
	div1.appendChild(br);
	var serverbartext = document.createElement('div');
	serverbartext.className = 'serverbartext';
	serverbartext.id = 'serverbartext';
	div1.appendChild(serverbartext);

        var div2 = document.createElement('div');
	var server_name = document.createElement('div');
	server_name.id = 'server_name';
	server_name.innerHTML = '';
	div2.appendChild(server_name);
        div2.className = 'channelbar';
	var channel_list = document.createElement('div');
	channel_list.id = 'channel_list';
	channel_list.className = 'channel_list';
	div2.appendChild(channel_list);
        document.body.appendChild(div2);

        var div3 = document.createElement('div');
        div3.innerHTML = '&nbsp;';
        div3.className = 'messagebar';

	var input_div = document.createElement('div');
	var textarea = document.createElement('input');
	textarea.type = 'text';
	textarea.className = 'message_input';

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
	chat_area.className = 'chat_area';
	chat_area.id = 'chat_area';
	div3.appendChild(input_div);
	input_div.appendChild(textarea);
	input_div.appendChild(chat_area);
	input_div.className = 'input_div';

        document.body.appendChild(div3);

        var div4 = document.createElement('div');
        div4.innerHTML = '';
        div4.className = 'statusbar';
	div4.id = 'statusbar';
        document.body.appendChild(div4);

	load_server_bar();
}

function process_response(response) {
	var event = JSON.parse(response);
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
		var chat_area = document.getElementById('chat_area');
		var m = document.createTextNode(
			event.user_pubkey.substring(0, 10) +
			'> ' +
			event.ebody
		);
		chat_area.appendChild(m);
		chat_area.appendChild(document.createElement('br'));
		chat_area.scrollTop = chat_area.scrollHeight;
	} else if (event.etype == 3) { // pong complete
		listener();
	}
 
}

function listener() {
	var req = new XMLHttpRequest();
	var start = 0;
	req.addEventListener("progress", function() {
		var resp = this.responseText.substring(start);
		var end = resp.indexOf("-----BREAK\r\n");
		var response = resp.substring(0, end);
		console.info('response='+response);
		try {
			process_response(response);
		} catch(ex) {
			console.error('exception: ' + ex + ',response='+response);
		}
		start += end + 12;
	});
	var rand = makeid(8);
	req.open("GET", '/listen?r='+rand);
	req.send();
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

function load_members(sname) {
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
 		var members = JSON.parse(this.responseText);
                var status_bar = document.getElementById('statusbar');
                status_bar.innerHTML = '';
                members.forEach(function(member) {
			if (member.user_type == 1) {
				status_bar.appendChild(
					document.createTextNode(
						member.user_pubkey.substring(0, 10) +
						" [owner]"
					)
				);
			} else {
                                status_bar.appendChild(
                                        document.createTextNode(
                                                member.user_pubkey.substring(0, 10)
                                        )
                                );
			}
			status_bar.appendChild(document.createElement('br'));
		});
	});
	req.open("GET", '/get_members?server_id='+sname);
	req.send();
}

window.onunload = function(event) { 
	var req = new XMLHttpRequest();
	req.open("GET", '/disconnect?listener_id='+listener_id);
	req.send();
}

