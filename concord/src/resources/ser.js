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

		var str16 = v.toString(16);
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
		var len = new U64(str.length);
		// len + bytes for storing len
		var ret = new ArrayBuffer(str.length + 8);
		var ret = new Uint8Array(ret);

		var ser_len = len.serialize(len);
		for(var i=0; i<8; i++) {
			ret[i] = ser_len[i];
		}

		var buffer = new Buffer(str, 'utf8');
		for(var i=0; i<str.length; i++) {
			ret[i+8] = buffer[i];
		}
		return ret;
        }

        deserialize(buffer, offset) {
		var len = U64.prototype.deserialize(buffer, offset);
		var ret = new SerString();
		var str_buffer = Uint8Array(len.value);
		for(var i=0; i<len.value; i++) {
			str_buffer[i] = buffer[8+offset+i];
		}
		ret.value = String.fromCharCode.apply(null, str_buffer);
		ret.offset = offset + 8 + len.value;
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

const EVENT_TYPE_AUTH      = 0;
const EVENT_TYPE_CHALLENGE = 1;
const EVENT_TYPE_AUTH_RESP = 2;

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

class Event {
	constructor(event_type, event_data) {
		if(event_type !== undefined) {
			this.version = EVENT_VERSION;
			this.event_type = event_type;
			if(this.event_type == EVENT_TYPE_AUTH) {
				this.auth_event = new SerOption(event_data);
			} else if(this.event_type == EVENT_TYPE_CHALLENGE) {
				this.challenge_event = new SerOption(event_data);
			} else if(this.event_type == EVENT_TYPE_AUTH_RESP) {
				this.auth_resp = new SerOption(event_data);
			} else {
				throw "Unknown event type = " + event_type;
			}
		}
	}

	serialize(event) {
		if(event.event_type == EVENT_TYPE_AUTH) {
			var x = event.auth_event.serialize(event.auth_event, AuthEvent.prototype);
			var y = new ArrayBuffer(x.length + 2);
			var y = new Uint8Array(y);
			y[0] = event.version;
			y[1] = EVENT_TYPE_AUTH;
			for(var i=0; i<x.length; i++) {
				y[i+2] = x[i];
			}
			return y;
		} else if(event.event_type == EVENT_TYPE_CHALLENGE) {
			var x = event.challenge_event.serialize(event.challenge_event, ChallengeEvent.prototype);
                        var y = new ArrayBuffer(x.length + 2);
			var y = new Uint8Array(y);
			y[0] = event.version;
                        y[1] = EVENT_TYPE_CHALLENGE;
                        for(var i=0; i<x.length; i++) {
                                y[i+2] = x[i];
                        }
                        return y;
		} else if(event.event_type == EVENT_TYPE_AUTH_RESP) {
                        var x = event.auth_resp.serialize(event.auth_resp, AuthResp.prototype);
                        var y = new ArrayBuffer(x.length + 2);
                        var y = new Uint8Array(y);
                        y[0] = event.version;
                        y[1] = EVENT_TYPE_AUTH_RESP;
                        for(var i=0; i<x.length; i++) {
                                y[i+2] = x[i];
                        }
                        return y;
		} else {
			throw "Unknown event type = " + event.event_type;
		}

	}

	deserialize(buffer) {
		var event = new Event();
		event.version = buffer[0];
		event.event_type = buffer[1];
		if(event.event_type == EVENT_TYPE_AUTH) {
			event.auth_event = SerOption
				.prototype
				.deserialize(buffer, 2, AuthEvent.prototype);
		} else if(event.event_type == EVENT_TYPE_CHALLENGE) {
			event.challenge_event = SerOption
				.prototype
				.deserialize(buffer, 2, ChallengeEvent.prototype);
		} else if(event.event_type == EVENT_TYPE_AUTH_RESP) {
			event.auth_resp = SerOption
				.prototype
				.deserialize(buffer, 2, AuthResp.prototype);
		} else {
			throw "Unknown event type = " + event.event_type;
		}

		return event;
	}
}


function array_buffer_to_event(buffer) {
	var event = new Event();
	var event2 = event.deserialize(buffer);
	return event2;
}

function event_to_array_buffer(event) {
	var buffer = Event.prototype.serialize(event);
	return buffer;
}

