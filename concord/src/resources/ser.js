/*
	Example using this:
                <script>
                        function getCookie(cname) {
                                let name = cname + "=";
                                let decodedCookie = decodeURIComponent(document.cookie);
                                let ca = decodedCookie.split(';');
                                for(let i = 0; i <ca.length; i++) {
                                        let c = ca[i];
                                        while (c.charAt(0) == ' ') {
                                                c = c.substring(1);
                                        }
                                        if (c.indexOf(name) == 0) {
                                                return c.substring(name.length, c.length);
                                        }
                                }
                                return "";
                        }

                        var token = BigInt(getCookie("auth"));
                        var sock = new WebSocket("ws://localhost:8093/ws");
                        sock.binaryType = "arraybuffer";
                        sock.onmessage = function (event) {
                                var event = new Event(
                                        EVENT_TYPE_AUTH,
                                        new AuthEvent(
                                                new SerOption(),
                                                new SerOption(
                                                        token,
                                                ),
                                                new SerOption(),
                                        )
                                );

                                let buffer = event.serialize(event);
                                sock.send(buffer);
                        }
                </script>
*/

function uint8ToBase64( bytes ) {
	var binary = '';
	var len = bytes.byteLength;
	for (var i = 0; i < len; i++) {
		binary += String.fromCharCode( bytes[ i ] );
	}
	return window.btoa( binary );
}

var EVENT_VERSION = 1;

// u128
BigInt.prototype.serialize = function(bint) {
        var buffer = new ArrayBuffer(16);
	var buffer = new Uint8Array(buffer);
        for(var i=0; i<16; i++) buffer[i] = 0;
        var str16 = bint.toString(16);
        var len = str16.length;
        if(len % 2 != 0) {
                str16 = '0' + str16;
                len++;
        }


        var itt = 15;
        for(var i=len-2; i>=0; i-=2) {
                var hex = str16.substring(i, i+2);
                var num = parseInt(hex, 16);
                buffer[itt] = num;
                itt--;
        }
        return buffer;
};


// u128
BigInt.prototype.deserialize = function(buffer, offset) {
	var num = BigInt(0);
	var itt = 0;
	for(var i=15+offset; i>=offset; i--) {
		num += BigInt(buffer[i]) << (BigInt(itt) * 8n);
		itt++;
	}
	BigInt.prototype.offset = offset + 16;
	return num;
};

class U64 {
	constructor(big_int) {
		this.value = big_int;
	}

	serialize(bint) {
		var buffer = new ArrayBuffer(8);
		var buffer = new Uint8Array(buffer);

		for(var i=0; i<8; i++) {
			buffer[i] = 0;
		}

		var str16 = bint.toString(16);
		var len = str16.length;
		if(len % 2 != 0) {
			str16 = '0' + str16;
			len++;
		}
		var itt = 7;
		for(var i=len-2; i>=0; i-=2) {
			var hex = str16.substring(i, i+2);
			var num = parseInt(hex, 16);
			buffer[itt] = num;
			itt--;
		}

		return buffer;
	}

	deserialize(buffer, offset) {
        	var num = BigInt(0);
        	var itt = 0;
        	for(var i=7+offset; i>=offset; i--) {
                	num += BigInt(buffer[i]) << (BigInt(itt) * 8n);
                	itt++;
        	}
        	U64.prototype.offset = offset + 8;
        	return new U64(num);
	}
}

class SerString {
	constructor(value) {
		this.value = value;
	}

        serialize(str) {
		var ret = new ArrayBuffer(str.length + 8);
		var ret = new Uint8Array(ret);

		var ser_len = U64.prototype.serialize(str.length);
		for(var i=0; i<8; i++) {
			ret[i] = ser_len[i];
		}


		const encoder = new TextEncoder();
		var buffer = encoder.encode(str);
		for(var i=0; i<str.length; i++) {
			ret[i+8] = buffer[i];
		}
		return ret;
        }

        deserialize(buffer, offset) {
		var len = U64.prototype.deserialize(buffer, offset);
		var ret = new SerString();
		var str_buffer = new Uint8Array(Number(len.value));
		for(var i=0; i<len.value; i++) {
			str_buffer[i] = buffer[8+offset+i];
		}
		ret.value = String.fromCharCode.apply(null, str_buffer);
		ret.offset = offset + 8 + Number(len.value);
		return ret;
        }
}

class SerOption {
	constructor(value) {
		this.value = value;
	}

	serialize(ser_option, serializer) {
		if(ser_option.value === undefined) {
			var b = new ArrayBuffer(1);
			var b = new Uint8Array(b);

			b[0] = 0;
			return b;
		} else {
			var x = serializer.serialize(ser_option.value);
			var ylen = 1 + x.length;
			var y = new ArrayBuffer(ylen);
			var y = new Uint8Array(y);

			y[0] = 1;
			for(var i=0; i<x.length; i++) {
				y[i+1] = x[i];
			}
			return y;
		}
	}

	deserialize(buffer, offset, deserializer) {
		var ser_option = new SerOption();
		if(buffer[offset] != 0) {
			ser_option.value = deserializer.deserialize(buffer, offset+1);
			ser_option.offset = ser_option.value.offset;
		} else {
			ser_option.offset = offset + 1;
		}
		return ser_option;
	}
}

class Icon {
        constructor(value) {
                this.value = value;
        }

        serialize(data) {
                var len = new U64(data.length);
                // len + bytes for storing len
                var ret = new ArrayBuffer(data.length + 8);
                var ret = new Uint8Array(ret);

                var ser_len = len.serialize(len);
                for(var i=0; i<8; i++) {
                        ret[i] = ser_len[i];
                }

                for(var i=0; i<data.length; i++) {
                        ret[i+8] = data[i];
                }
                return ret;
        }

        deserialize(buffer, offset) {
                var len = U64.prototype.deserialize(buffer, offset);
                var ret = new Icon();
                var data = new Uint8Array(Number(len.value));
                for(var i=0; i<len.value; i++) {
                        data[i] = buffer[8+offset+i];
                }
                ret.value = data;
                ret.offset = offset + 8 + Number(len.value);
                return ret;
        }
}

class Pubkey {
	constructor() {
	}

	serialize(pubkey) {
		var ret = new ArrayBuffer(32);
		var ret = new Uint8Array(ret);

		for(var i=0; i<32; i++) {
			ret[i] = pubkey.data[i];
		}
		return ret;
	}

	deserialize(buffer, offset) {
		var pubkey = new Pubkey();
		pubkey.data = [];
		for(var i=offset; i<offset+32; i++) {
			pubkey.data[i-offset] = buffer[i];
		}
		pubkey.offset = offset + 32;
		return pubkey;
	}
}

class ServerId {
        constructor() {
        }

        serialize(server_id) {
                var ret = new ArrayBuffer(8);
                var ret = new Uint8Array(ret);

                for(var i=0; i<8; i++) {
                        ret[i] = server_id.data[i];
                }
                return ret;
        }

        deserialize(buffer, offset) {
                var server_id= new ServerId();
		server_id.data = []
                for(var i=offset; i<offset+8; i++) {
                        server_id.data[i-offset] = buffer[i];
                }
                server_id.offset = offset + 8;
                return server_id;
        }
}

class Signature {
	constructor() {
	}

	serialize(signature) {
                var ret = new ArrayBuffer(64);
		var ret = new Uint8Array(ret);
                for(var i=0; i<64; i++) {
                        ret[i] = signature.data[i];
                }
                return ret;
	}

	deserialize(buffer, offset) {
		var signature = new Signature();
		signature.data = [];
		for(var i=offset; i<offset+64; i++) {
			signature.data[i-offset] = buffer[i];
		}
		signature.offset = offset + 64;
		return signature;
	}
}

// note that these must match with the server codes
const EVENT_TYPE_AUTH                 = 0;
const EVENT_TYPE_CHALLENGE            = 1;
const EVENT_TYPE_AUTH_RESP            = 2;
const EVENT_TYPE_GET_SERVERS_EVENT    = 3;
const EVENT_TYPE_GET_SERVERS_RESPONSE = 4;
const EVENT_TYPE_CREATE_SERVER_EVENT  = 5;
const EVENT_TYPE_DELETE_SERVER_EVENT  = 6;

class DeleteServerEvent {
	constructor(server_id, server_pubkey) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
	}

	serialize(delete_server_event) {
		var ret = new Uint8Array(new ArrayBuffer(40));
		for(var i=0; i<32; i++)
			ret[i] = delete_server_event.server_pubkey[i];
		for(var i=0; i<8; i++)
			ret[i+32] = delete_server_event.server_id[i];
		return ret;
	}

	deserialize(buffer, offset) {
		throw "TODO: implement DeleteServerEvent.deserialize";
	}
}

class CreateServerEvent {
	constructor(name, icon) {
		this.name = name;
		this.icon = icon;
	}

	serialize(create_server_event) {
                var x = SerString.prototype.serialize(create_server_event.name);
		var ser_len = U64.prototype.serialize(create_server_event.icon.length);
		var ret = new ArrayBuffer(x.length + 8 + create_server_event.icon.length);
		var ret = new Uint8Array(ret);
		for(var i=0; i<x.length; i++)
			ret[i] = x[i];
		for(var i=0; i<8; i++)
			ret[i+x.length] = ser_len[i];
		for(var i=0; i<create_server_event.icon.length; i++)
			ret[i+x.length+8] = create_server_event.icon[i];
                return ret;
	}

	deserialize(buffer, offset) {
		throw "TODO: implement CreateServerEvent.deserialize";
	}
}

class GetServersEvent {
	constructor() {
	}

	serialize(get_servers_event) {
		var ret = new ArrayBuffer(0);
		return ret;
	}

	deserialize(buffer, offset) {
		throw "TODO: implement GetServersEvent.deserialize";
	}
}

class ChallengeEvent {
	constructor(challenge) {
		this.challenge = challenge;
	}

	serialize(challenge_event) {
		var ret = challenge_event.challenge.serialize(challenge_event.challenge);
		return ret;
	}

	deserialize(buffer, offset) {
		var challenge_event = new ChallengeEvent();
		challenge_event.challenge = BigInt.prototype.deserialize(buffer, offset);
		challenge_event.offset = offset + 16;
		return challenge_event;
	}
}

class AuthResp {
	constructor(success, redirect) {
		this.success = success;
		this.redirect = redirect;
	}

	serialize(auth_resp) {
		var x = auth_resp.redirect.serialize(auth_resp.redirect, SerString.prototype);
		var ret = new ArrayBuffer(x.length + 1);
		ret[0] = auth_resp.success;
                for(var i=0; i<x.length; i++) {
                        ret[i+1] = x[i];
		}

		return ret;
	}

	deserialize(buffer, offset) {
		var auth_resp = new AuthResp();
		auth_resp.success = buffer[offset] != 0;
		auth_resp.redirect = SerOption.prototype.deserialize(buffer, offset+1, SerString.prototype);
		return auth_resp;
	}
}

class AuthEvent {
	constructor(signature, token, pubkey) {
		this.signature = signature;
		this.token = token;
		this.pubkey = pubkey;
	}

	serialize(auth_event) {
		var x = auth_event.signature.serialize(auth_event.signature, Signature.prototype);
		var y = auth_event.token.serialize(auth_event.token, BigInt.prototype);
		var z = auth_event.pubkey.serialize(auth_event.pubkey, Pubkey.prototype);
		var ret = new ArrayBuffer(x.length + y.length + z.length);
		var ret = new Uint8Array(ret);
		var offset = 0;
		for(var i=0; i<x.length; i++) {
                	ret[offset] = x[i];
			offset++;
                }
		for(var i=0; i<y.length; i++) {
			ret[offset] = y[i];
			offset++;
		}
                for(var i=0; i<z.length; i++) {
                        ret[offset] = z[i];
                        offset++;
                }

		return ret;
	}

	deserialize(buffer, offset) {
		var auth_event = new AuthEvent();
		auth_event.signature = SerOption.prototype.deserialize(buffer, offset, Signature.prototype);
		var offset = auth_event.signature.offset;
		auth_event.token = SerOption.prototype.deserialize(buffer, offset, BigInt.prototype);
		var offset = auth_event.token.offset;
		auth_event.pubkey = SerOption.prototype.deserialize(buffer, offset, Pubkey.prototype);

		return auth_event;
	}
}

class ServerInfo {
	constructor(name, description, server_id, server_pubkey, icon) {
		this.name = name;
		this.description = description;
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.icon = icon;
	}
}

class GetServersResponse {
	serialize(get_servers_response) {
		throw "TODO: implement GetServersResponse.serialize";
	}

	deserialize(buffer, offset) {
		var servers_response = new GetServersResponse();
		var len = U64.prototype.deserialize(buffer, offset).value;
		offset += 8;
		servers_response.servers = [];
		for(var i=0; i<len; i++) {
			var name = SerString.prototype.deserialize(buffer, offset);
			offset = name.offset;
			var description = SerString.prototype.deserialize(buffer, offset);
			offset = description.offset;
			var server_id = ServerId.prototype.deserialize(buffer, offset);
			offset = offset + 8;
			var server_pubkey = Pubkey.prototype.deserialize(buffer, offset);
			offset = offset + 32;
			var icon = Icon.prototype.deserialize(buffer, offset);
			offset = icon.offset;
			servers_response.servers.push(
				new ServerInfo(
					name,
					description,
					server_id,
					server_pubkey,
					icon
				)
			);
		}
		servers_response.offset = offset;

		return servers_response;
	}
}

class Event {
	constructor(event_type, event_data) {
		if(event_type !== undefined) {
			this.version = EVENT_VERSION;
			this.timestamp = Date.now();
			this.event_type = event_type;
			if(this.event_type == EVENT_TYPE_AUTH) {
				this.auth_event = new SerOption(event_data);
			} else if(this.event_type == EVENT_TYPE_CHALLENGE) {
				this.challenge_event = new SerOption(event_data);
			} else if(this.event_type == EVENT_TYPE_AUTH_RESP) {
				this.auth_resp = new SerOption(event_data);
			} else if(this.event_type == EVENT_TYPE_GET_SERVERS_EVENT) {
				this.get_servers_event = new SerOption(new GetServersEvent());
			} else if(this.event_type == EVENT_TYPE_CREATE_SERVER_EVENT) {
				this.create_server_event = new SerOption(event_data);
			} else if(this.event_type == EVENT_TYPE_DELETE_SERVER_EVENT) {
				this.delete_server_event = new SerOption(event_data);
			} else {
				throw "Unknown event in Event.constructor type = " + event_type;
			}
		}
	}

	serialize(event) {
		var x;
		if(event.event_type == EVENT_TYPE_AUTH) {
			x = event.auth_event.serialize(event.auth_event, AuthEvent.prototype);
		} else if(event.event_type == EVENT_TYPE_CHALLENGE) {
			x = event.challenge_event.serialize(event.challenge_event, ChallengeEvent.prototype);
		} else if(event.event_type == EVENT_TYPE_AUTH_RESP) {
			x = event.auth_resp.serialize(event.auth_resp, AuthResp.prototype);
		} else if(event.event_type == EVENT_TYPE_GET_SERVERS_EVENT) {
			x = new Uint8Array(1);
			x[0] = 1;
		} else if(event.event_type == EVENT_TYPE_CREATE_SERVER_EVENT) {
			x = event.create_server_event.serialize(event.create_server_event, CreateServerEvent.prototype);
		} else if(event.event_type == EVENT_TYPE_DELETE_SERVER_EVENT) {
			x = event.delete_server_event.serialize(event.delete_server_event, DeleteServerEvent.prototype);
		} else {
			throw "Unknown event type in event.serialize = " + event.event_type;
		}

		var ret = new ArrayBuffer(x.length + 18);
		var ret = new Uint8Array(ret);
		ret[0] = event.version;
		var t = BigInt.prototype.serialize(event.timestamp);
		for(var i=0; i<16; i++) {
			ret[i+1] = t[i];
		}
		ret[17] = event.event_type;
		for(var i=0; i<x.length; i++) {
			ret[i+18] = x[i];
		}
		return ret;

	}

	deserialize(buffer) {
		var event = new Event();
		event.version = buffer[0];
		event.timestamp = BigInt.prototype.deserialize(buffer, 1);
		event.event_type = buffer[17];
		if(event.event_type == EVENT_TYPE_AUTH) {
			event.auth_event = SerOption
				.prototype
				.deserialize(buffer, 18, AuthEvent.prototype);
		} else if(event.event_type == EVENT_TYPE_CHALLENGE) {
			event.challenge_event = SerOption
				.prototype
				.deserialize(buffer, 18, ChallengeEvent.prototype);
		} else if(event.event_type == EVENT_TYPE_AUTH_RESP) {
			event.auth_resp = SerOption
				.prototype
				.deserialize(buffer, 18, AuthResp.prototype);
		} else if(event.event_type == EVENT_TYPE_GET_SERVERS_EVENT) {
			event.get_servers = SerOption
				.prototype
				.deserialize(buffer, 18, GetServers.prototype);
		} else if(event.event_type == EVENT_TYPE_GET_SERVERS_RESPONSE) {
			event.get_servers_response = SerOption
				.prototype
				.deserialize(buffer, 18, GetServersResponse.prototype);
		} else if(event.event_type == EVENT_TYPE_CREATE_SERVER_EVENT) {
			event.create_server_event = SerOption
				.prototype
				.deserialize(buffer, 18, CreateServerEvent.prototype);	
		} else if(event.event_type == EVENT_TYPE_DELETE_SERVER_EVENT) {
			event.delete_server_event = SerOption
				.prototype
				.deserialize(buffer, 18, DeleteServerEvent.prototype);
		} else {
			throw "Unknown event type in event.deserialize = " + event.event_type;
		}

		return event;
	}
}


function array_buffer_to_event(buffer) {
	return Event.prototype.deserialize(buffer);
}

function event_to_array_buffer(event) {
	var buffer = Event.prototype.serialize(event);
	return buffer;
}

