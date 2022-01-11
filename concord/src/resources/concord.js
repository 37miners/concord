// concord's own js functions

var cur_server = '';
var cur_channel = '';

var stopEnabled = false;
var iconId = '';
var curName = '';
var menu = [{
            name: 'Invite',
            img: 'images/create.png',
            fun: function () {
                alert('i am a create button')
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
                req.open("GET", '/delete_server?id='+iconId);
                req.send();
            }
}];

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
}

function show_auth_error() {
	document.getElementById('interstitial').style.visibility = 'visible';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
}

function create_server() {
	document.getElementById("interstitialtextpadding2").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
}

function join_server() {
        document.getElementById("interstitialtextpadding3").style.visibility = 'visible';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding4").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
}

function modify_server(iconId, curName) {
	var rand = makeid(8);
	document.getElementById('curImage').src = '/get_server_icon?id=' + iconId + '&r=' + rand;
	document.forms['modify']['id'].value = iconId;
	document.forms['modify']['name'].value = curName;
	document.getElementById('interstitial').style.visibility = 'visible';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding4").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding5").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding6").style.visibility = 'hidden';
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
                req.open("POST", '/modify_server?id=' + iconId + '&name='+encodeURIComponent(name));
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
}

function load_server_bar() {
	// first clear server bar
	document.getElementById('serverbartext').innerHTML = '';

	// then load all servers
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		var servers = JSON.parse(this.responseText);
		if (servers.error !== undefined) {
			// authentication error
			show_auth_error();
		} else {
			servers.forEach(function(server) {
				var serverbar = document.getElementById('serverbartext');
				var img = document.createElement('img');
				var rand = makeid(8);
				img.src = '/get_server_icon?id=' + server.id + '&r=' + rand;
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
								var req = new XMLHttpRequest();
								req.addEventListener("load", function() {
									var chat_area = document.getElementById('chat_area');
									chat_area.innerHTML = '';
									var messages = JSON.parse(this.responseText);
									messages.forEach(function(message) {
										var m = document.createTextNode('> '+message.text);
										chat_area.appendChild(m);
										chat_area.appendChild(document.createElement('br'));
									});
								});
								req.open(
									"GET",
									'/query_messages?server_id='+server.id+
									'&channel_id='+channel.id
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
					req.open("GET", '/get_channels?server_id='+server.id);
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
        div4.innerHTML = 'statusbar';
        div4.className = 'statusbar';
        document.body.appendChild(div4);

	load_server_bar();
}

