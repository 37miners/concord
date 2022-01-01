// concord's own js functions

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
			//alert(server.name + ' ' + server.address);
			var serverbar = document.getElementById('serverbartext');
			/*
			serverbar.appendChild(
				document.createTextNode(
					server.name + ' ' + server.address
				)
			);
			*/
			//serverbar.appendChild(document.createElement('br'));
			var img = document.createElement('img');
			img.src = '/get_server_icon?id=' + server.id;
			img.className = 'server_icon';
			img.title = server.name;
			serverbar.appendChild(img);
			serverbar.appendChild(document.createElement('br'));
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
		req.open("POST", 'create_server?name='+name);
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

