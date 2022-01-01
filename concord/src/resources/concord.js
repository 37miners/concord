// concord's own js functions

var stopEnabled = false;
var iconId = '';
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
                alert('i am an update button')
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

document.oncontextmenu = function(e){
	if(stopEnabled) {
		stopEvent(e);
	}
}
function stopEvent(event){
	if(event.preventDefault != undefined)
		event.preventDefault();
	if(event.stopPropagation != undefined)
		event.stopPropagation();
}

function close_interstitial() {
	document.getElementById('interstitial').style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
}

function create_server() {
	document.getElementById("interstitialtextpadding2").style.visibility = 'visible';
	document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
	document.getElementById("interstitialtextpadding3").style.visibility = 'hidden';
}

function join_server() {
        document.getElementById("interstitialtextpadding3").style.visibility = 'visible';
        document.getElementById("interstitialtextpadding1").style.visibility = 'hidden';
        document.getElementById("interstitialtextpadding2").style.visibility = 'hidden';
}

function load_server_bar() {
	// first clear server bar
	document.getElementById('serverbartext').innerHTML = '';

	// then load all servers
	var req = new XMLHttpRequest();
	req.addEventListener("load", function() {
		var servers = JSON.parse(this.responseText);
		servers.forEach(function(server) {
			var serverbar = document.getElementById('serverbartext');
			var img = document.createElement('img');
			img.src = '/get_server_icon?id=' + server.id;
			img.className = 'server_icon';
			img.title = decodeURIComponent(server.name);
			img.id = server.id;
			img.onmouseover = function() {
				stopEnabled = true;
				iconId = server.id;
			};
			img.onmouseout = function() {
				stopEnabled = false;
			};

			serverbar.appendChild(img);
			serverbar.appendChild(document.createElement('br'));
			$('.server_icon').contextMenu(menu, {triggerOn:'contextmenu'});
		});
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
		req.open("POST", 'create_server?name='+encodeURIComponent(name));
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
        div2.innerHTML = 'channelbar';
        div2.className = 'channelbar';
        document.body.appendChild(div2);

        var div3 = document.createElement('div');
        div3.innerHTML = '&nbsp;';
        div3.className = 'messagebar';
        document.body.appendChild(div3);

        var div4 = document.createElement('div');
        div4.innerHTML = 'statusbar';
        div4.className = 'statusbar';
        document.body.appendChild(div4);

	load_server_bar();
}

