function uint8ToBase64( bytes ) {
	var binary = '';
	var len = bytes.byteLength;
	for (var i = 0; i < len; i++) {
		binary += String.fromCharCode( bytes[ i ] );
	}
	return window.btoa( binary );
}

var EVENT_VERSION = 1;

class U128 {
        constructor(big_int) {
                this.value = big_int;
        }

        serialize(bint) {
                var buffer = new ArrayBuffer(16);
                var buffer = new Uint8Array(buffer);

                for(var i=0; i<16; i++) {
                        buffer[i] = 0;
                }

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
        }

        deserialize(buffer, offset) {
                var num = BigInteger.ZERO;
                var itt = 0;
                for(var i=15+offset; i>=offset; i--) {
			num = num.add(
				new BigInteger(
					String(buffer[i]),
					10
				).shiftLeft(
					new BigInteger(
						String(itt),
						10
					).multiply(
						new BigInteger("8", 10)
					)
				)
			);
                        itt++;
                }
                var ret = new U128(num);
                ret.offset = offset + 16;
                return ret;
        }
}

class U32 {
	constructor(value) {
		this.value = value;
	}

        serialize(bint) {
                var buffer = new ArrayBuffer(4);
                var buffer = new Uint8Array(buffer);
                
                for(var i=0; i<4; i++) {
                        buffer[i] = 0; 
                }       
                
                var str16 = bint.toString(16);
                var len = str16.length;
                if(len % 2 != 0) {
                        str16 = '0' + str16;
                        len++;
                }       
                var itt = 3;
                for(var i=len-2; i>=0; i-=2) {
                        var hex = str16.substring(i, i+2);
                        var num = parseInt(hex, 16); 
                        buffer[itt] = num;
                        itt--;
                }       
                
                return buffer;
        }       
        
        deserialize(buffer, offset) {
                var num = BigInteger.ZERO;
                var itt = 0;
                for(var i=3+offset; i>=offset; i--) {
                        num = num.add(
                                new BigInteger(
                                        String(buffer[i]),
                                        10
                                ).shiftLeft(
                                        new BigInteger(
                                                String(itt),
                                                10
                                        ).multiply(
                                                new BigInteger("8", 10)
                                        )       
                                )       
                        );      
                        itt++;  
                }       
                var ret = new U32(num);
                ret.offset = offset + 4;
                return ret;
        }
}

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
		var num = BigInteger.ZERO;
		var itt = 0;
		for(var i=7+offset; i>=offset; i--) {
			num = num.add(
				new BigInteger(
					String(buffer[i]),
					10
				).shiftLeft(
					new BigInteger(
						String(itt), 
						10
					).multiply(
						new BigInteger("8", 10)
					)
				)
			);
			itt++;
        	}
		var ret = new U64(num);
        	ret.offset = offset + 8;
        	return ret;
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
	constructor(data) {
		this.data = data;
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
        constructor(data) {
		this.data = data;
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
const EVENT_TYPE_AUTH                    = 0;
const EVENT_TYPE_CHALLENGE               = 1;
const EVENT_TYPE_AUTH_RESP               = 2;
const EVENT_TYPE_GET_SERVERS_EVENT       = 3;
const EVENT_TYPE_GET_SERVERS_RESPONSE    = 4;
const EVENT_TYPE_CREATE_SERVER_EVENT     = 5;
const EVENT_TYPE_DELETE_SERVER_EVENT     = 6;
const EVENT_TYPE_MODIFY_SERVER_EVENT     = 7;
const EVENT_TYPE_GET_CHANNELS_REQUEST    = 8;
const EVENT_TYPE_GET_CHANNELS_RESPONSE   = 9;
const EVENT_TYPE_DELETE_CHANNEL_REQUEST  = 10;
const EVENT_TYPE_DELETE_CHANNEL_RESPONSE = 11;
const EVENT_TYPE_MODIFY_CHANNEL_REQUEST  = 12;
const EVENT_TYPE_MODIFY_CHANNEL_RESPONSE = 13;
const EVENT_TYPE_ADD_CHANNEL_REQUEST     = 14;
const EVENT_TYPE_ADD_CHANNEL_RESPONSE    = 15;
const EVENT_TYPE_GET_MEMBERS_REQUEST     = 16;
const EVENT_TYPE_GET_MEMBERS_RESPONSE    = 17;
const EVENT_TYPE_CREATE_INVITE_REQUEST   = 18;
const EVENT_TYPE_CREATE_INVITE_RESPONSE  = 19;
const EVENT_TYPE_LIST_INVITES_REQUEST    = 20;
const EVENT_TYPE_LIST_INVITES_RESPONSE   = 21;
const EVENT_TYPE_MODIFY_INVITE_REQUEST   = 22;
const EVENT_TYPE_MODIFY_INVITE_RESPONSE  = 23;
const EVENT_TYPE_DELETE_INVITE_REQUEST   = 24;
const EVENT_TYPE_DELETE_INVITE_RESPONSE  = 25;
const EVENT_TYPE_SET_PROFILE_REQUEST     = 34;
const EVENT_TYPE_SET_PROFILE_RESPONSE    = 35;

const FIRST_EVENT_DATA = 23; // first byte of event data

class ProfileData {
	constructor(user_name, user_bio) {
		this.user_name = user_name;
		this.user_bio = user_bio;
	}

	serialize(profile_data) {
		var a = SerString.prototype.serialize(profile_data.user_name);
		var b = SerString.prototype.serialize(profile_data.user_bio);
		var ret = new Uint8Array(new ArrayBuffer(a.length + b.length));
		for(var i=0; i<a.length; i++)
			ret[i] = a[i];
		for(var i=0; i<b.length; i++)
			ret[i+a.length] = b[i];

		return ret;

	}

	deserialize() {
		throw "TODO: implement ProfileData.deserialize";
	}
}

class SetProfileRequest {
	constructor(server_pubkey, server_id, avatar, profile_data) {
		this.server_pubkey = server_pubkey;
		this.server_id = server_id;
		this.avatar = avatar;
		this.profile_data = profile_data;
	}

        serialize(set_profile_request) {
		var a = Pubkey.prototype.serialize(set_profile_request.server_pubkey);
		var b = ServerId.prototype.serialize(set_profile_request.server_id);
		var c = SerOption.prototype.serialize(set_profile_request.avatar, Icon.prototype);
                var d = SerOption.prototype.serialize(set_profile_request.profile_data, ProfileData.prototype);
		var ret = new Uint8Array(new ArrayBuffer(a.length + b.length + c.length + d.length));
                for(var i=0; i<a.length; i++)
                        ret[i] = a[i];
                for(var i=0; i<b.length; i++)
                        ret[i+a.length] = b[i];
                for(var i=0; i<c.length; i++)
                        ret[i+a.length+b.length] = c[i];
		for(var i=0; i<d.length; i++)
			ret[i+a.length+b.length+c.length] = d[i];

                return ret;
        }

        deserialize() {
                throw "TODO: implement SetProfileRequest.deserialize";
        }
}

class Invite {
	constructor(invite_id, max, current, expiration, server_id, inviter) {
		this.invite_id = invite_id;
		this.max = max;
		this.current = current;
		this.expiration = expiration;
		this.server_id = server_id;
		this.inviter = inviter;
	}

	serialize() {
		throw "TODO: implement Invite.serialize";
	}

	deserialize(buffer, offset) {
                var server_id = ServerId.prototype.deserialize(buffer, offset);
                offset += 8;
		var inviter = Pubkey.prototype.deserialize(buffer, offset);
		offset += 32;
		var expiration = U128.prototype.deserialize(buffer, offset);
		offset += 16;
                var current = U64.prototype.deserialize(buffer, offset);
                offset += 8;
                var max = U64.prototype.deserialize(buffer, offset);
                offset += 8;
		var invite_id = U128.prototype.deserialize(buffer, offset);
		offset += 16;	
                var ret = new Invite(invite_id, max, current, expiration, server_id, inviter);
                ret.offset = offset;
                return ret;
	}
}

class ListInvitesResponse {
	constructor(invites) {
		this.invites = invites;
	}

	serialize() {
		throw "TODO: implement ListInvitesResponse.serialize";
	}

	deserialize(buffer, offset) {
		var invite_count = U64.prototype.deserialize(buffer, offset);
		offset += 8;
		var invites = [];
		for(var i=0; i<invite_count.value; i++) {
			var invite = Invite.prototype.deserialize(buffer, offset);
			offset = invite.offset;
			invites.push(invite);
		}
                var ret = new ListInvitesResponse(invites);
                ret.offset = offset;
                return ret;
	}
}

class DeleteInviteRequest {
	constructor(invite_id) {
		this.invite_id = invite_id;
	}

	serialize(delete_invite_request) {
		var ret = new Uint8Array(new ArrayBuffer(32));
		var invite_id = U128.prototype.serialize(delete_invite_request.invite_id);
		for(var i=0; i<16; i++)
			ret[i] = invite_id[i];
		return ret;
	}

	deserialize() {
		throw "TODO: implement DeleteInviteRequest.deserialize";
	}
}

class CreateInviteRequest {
	constructor(server_id, server_pubkey, count, expiration) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.count = count;
		this.expiration = expiration;
	}

	serialize(create_invite_request) {
                var ret = new Uint8Array(new ArrayBuffer(80));
                for(var i=0; i<8; i++)
                        ret[i] = create_invite_request.server_id[i];
                for(var i=0; i<32; i++)
                        ret[i+8] = create_invite_request.server_pubkey[i];
		var count = U64.prototype.serialize(create_invite_request.count);
		var expiration = U128.prototype.serialize(create_invite_request.expiration);
		for(var i=0; i<8; i++)
			ret[i+40] = count[i];
		for(var i=0; i<16; i++)
			ret[i+48] = expiration[i];
                return ret;	
	}

        deserialize() {
                throw "TODO: implement CreateInviteRequest.deserialize";
        }
}

class ListInvitesRequest {
	constructor(server_id, server_pubkey) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
	}

	serialize(list_invites_request) {
                var ret = new Uint8Array(new ArrayBuffer(56));
                for(var i=0; i<8; i++)
                        ret[i] = list_invites_request.server_id[i];
                for(var i=0; i<32; i++)
                        ret[i+8] = list_invites_request.server_pubkey[i];

                return ret;
	}

	deserialize() {
		throw "TODO: implement ListInvitesRequest.deserialize";
	}
}

class Member {
	constructor() {

	}

	serialize() {
		throw "TODO: implement Member.serialize";
	}

	deserialize(buffer, offset) {
		var ret = new Member();
		ret.user_pubkey = Pubkey.prototype.deserialize(buffer, offset);
		offset += 32;
		ret.user_name = SerString.prototype.deserialize(buffer, offset);
		offset = ret.user_name.offset;
		ret.user_bio = SerString.prototype.deserialize(buffer, offset);
		offset = ret.user_bio.offset;
		ret.roles = U128.prototype.deserialize(buffer, offset);
		offset += 16;
		ret.profile_seqno = U64.prototype.deserialize(buffer, offset);
		offset += 8;
		ret.online_status = buffer[offset] == 0;
		offset += 1;
		

		ret.offset = offset;
		return ret;
	}
}

class GetMembersResponse {
	constuctor(members, server_id, server_pubkey, batch_num) {
		this.members = members;
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.batch_num = batch_num;
	}

	serialize() {
		throw "TODO: implement GetMembersResponse.serialize";
	}

	deserialize(buffer, offset) {
		var member_len = U64.prototype.deserialize(buffer, offset);
		offset += 8;
		var members = [];
		for(var i=0; i<member_len.value; i++) {
			var member = Member.prototype.deserialize(buffer, offset);
			members.push(member);
			offset = member.offset;
		}
		var ret = new GetMembersResponse();
		ret.members = members;
		ret.server_id = ServerId.prototype.deserialize(buffer, offset);
		offset += 8;
		ret.server_pubkey = Pubkey.prototype.deserialize(buffer, offset);
		offset += 32;
		ret.batch_num = U64.prototype.deserialize(buffer, offset);
		offset += 8;
		ret.offset = offset;
		return ret;
	}
}

class GetMembersRequest {
	constructor(server_id, server_pubkey, batch_num) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.batch_num = batch_num;
	}

	serialize(get_members_request) {
		var x = ServerId.prototype.serialize(get_members_request.server_id, ServerId.prototype);
 		var y = Pubkey.prototype.serialize(get_members_request.server_pubkey, Pubkey.prototype);
 		var z = U64.prototype.serialize(get_members_request.batch_num);
 		var ret = new Uint8Array(new ArrayBuffer(x.length + y.length + z.length));
 		for(var i=0; i<x.length; i++)
 			ret[i] = x[i];
 		for(var i=0; i<y.length; i++)
 			ret[i+x.length] = y[i];
 		for(var i=0; i<z.length; i++)
 			ret[i+x.length+y.length] = z[i];
 		return ret;
	}

	deserialize() {
		throw "TODO: implement GetMembersRequest.deserialize";
	}
}

class ModifyChannelRequest {
	constructor(server_id, server_pubkey, channel_id, name, description) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.channel_id = channel_id;
		this.name = name;
		this.description = description;
	}

	serialize(modify_channel_request) {
		var x = U64.prototype.serialize(modify_channel_request.channel_id);
                var y = SerString.prototype.serialize(modify_channel_request.name);
                var z = SerString.prototype.serialize(modify_channel_request.description);
                var ret = new Uint8Array(new ArrayBuffer(64 + y.length + z.length));
                for(var i=0; i<8; i++)
                        ret[i] = modify_channel_request.server_id[i];
                for(var i=0; i<32; i++)
                        ret[i+8] = modify_channel_request.server_pubkey[i];
		
                for(var i=0; i<x.length; i++)
                        ret[i+40] = x[i];
                for(var i=0; i<y.length; i++)
                        ret[i+x.length+40] = y[i];
		for(var i=0; i<z.length; i++)
			ret[i+x.length+y.length+40] = z[i];
                return ret;
	}

	deserialize() {
		throw "TODO: implement ModifyChannelRequest.deserialize";
	}
}

class DeleteChannelRequest {
	constructor(server_id, server_pubkey, channel_id) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.channel_id = channel_id;
	}

	serialize(delete_channel_request) {
                var ret = new Uint8Array(new ArrayBuffer(64));
                for(var i=0; i<8; i++)
                        ret[i] = delete_channel_request.server_id[i];
                for(var i=0; i<32; i++)
                        ret[i+8] = delete_channel_request.server_pubkey[i];
		var x = U64.prototype.serialize(delete_channel_request.channel_id);
		for(var i=0; i<8; i++)
			ret[i+40] = x[i];
                return ret;
	}

	deserialize() {
		throw "TODO: implement DeleteChannelRequest.deserialize";
	}
}

class AddChannelRequest {
	constructor(server_id, server_pubkey, name, description) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.name = name;
		this.description = description;
	}

	serialize(add_channel_request) {
		var x = SerString.prototype.serialize(add_channel_request.name);
		var y = SerString.prototype.serialize(add_channel_request.description);
                var ret = new Uint8Array(new ArrayBuffer(56 + x.length + y.length));
		for(var i=0; i<8; i++)
			ret[i] = add_channel_request.server_id[i];
                for(var i=0; i<32; i++)
                        ret[i+8] = add_channel_request.server_pubkey[i];
		for(var i=0; i<x.length; i++)
			ret[i+40] = x[i];
		for(var i=0; i<y.length; i++)
			ret[i+x.length+40] = y[i];
                return ret;
	}

	deserialize() {
		throw "TODO: implement AddChannelRequest.deserialize";
	}
}

class Channel {
	constructor(channel_id, name, description, server_id, server_pubkey) {
		this.channel_id = channel_id;
		this.name = name;
		this.description = description;
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
	}

	serialize() {
		throw "TODO: implement Channel.serialize";
	}

	deserialize(buffer, offset) {
		var channel_id = U64.prototype.deserialize(buffer, offset);
		offset += 8;
		var name = SerString.prototype.deserialize(buffer, offset);
		offset = name.offset;
		var description = SerString.prototype.deserialize(buffer, offset);
		offset = description.offset;
		var ret = new Channel(channel_id, name, description);
		ret.offset = offset;
		return ret;
	}
}

class GetChannelsResponse {
	serialize() {
		throw "TODO: implement GetChannelsResponse.serialize";
	}

	deserialize(buffer, offset) {
		var server_id = ServerId.prototype.deserialize(buffer, offset);
		offset = server_id.offset;
		var server_pubkey = Pubkey.prototype.deserialize(buffer, offset);
		offset = server_pubkey.offset;
		var len = U64.prototype.deserialize(buffer, offset).value;
		offset += 8;
		var channels = [];
		for(var i=0; i<len; i++) {
			var channel = Channel.prototype.deserialize(buffer, offset);
			offset = channel.offset;
			channels.push(channel);
		}

		var ret = new GetChannelsResponse();
		ret.channels = channels;
		ret.server_pubkey = server_pubkey;
		ret.server_id = server_id;
		return ret;
	}
}

class GetChannelsRequest {
	constructor(server_id, server_pubkey) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
	}

	serialize(get_channels_request) {
		var ret = new Uint8Array(new ArrayBuffer(40));
		for(var i=0; i<8; i++)
			ret[i] = get_channels_request.server_id[i];
		for(var i=0; i<32; i++)
			ret[i+8] = get_channels_request.server_pubkey[i];
		return ret;
	}

        deserialize(buffer, offset) {
                throw "TODO: implement GetChannelsRequest.deserialize";
        }
}

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

class ModifyServerEvent {
	constructor(server_id, server_pubkey, name, icon) {
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.name = name;
		this.icon = icon;
	}

	serialize(modify_server_event) {
		var w = SerOption.prototype.serialize(modify_server_event.name, SerString.prototype);
		var x = SerOption.prototype.serialize(modify_server_event.icon, Icon.prototype);
		var y = ServerId.prototype.serialize(modify_server_event.server_id, ServerId.prototype);
		var z = Pubkey.prototype.serialize(modify_server_event.server_pubkey, Pubkey.prototype);

		var ret = new Uint8Array(new ArrayBuffer(w.length + x.length + y.length + z.length));

		for(var i=0; i<w.length; i++) {
			ret[i] = w[i];
		}
		for(var i=0; i<x.length; i++) {
			ret[i+w.length] = x[i];
		}
		for(var i=0; i<y.length; i++) {
			ret[i+w.length + x.length] = y[i];
		}
		for(var i=0; i<z.length; i++) {
			ret[i+w.length+x.length+y.length] = z[i];
		}

		return ret;
	}

	deserialize(buffer, offset) {
		throw "TODO: implement ModServerEvent.deserialize";
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
		challenge_event.challenge = U128.prototype.deserialize(buffer, offset);
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
		var y = auth_event.token.serialize(auth_event.token, U128.prototype);
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
		auth_event.token = SerOption.prototype.deserialize(buffer, offset, U128.prototype);
		var offset = auth_event.token.offset;
		auth_event.pubkey = SerOption.prototype.deserialize(buffer, offset, Pubkey.prototype);

		return auth_event;
	}
}

class ServerInfo {
	constructor(name, description, server_id, server_pubkey, seqno) {
		this.name = name;
		this.description = description;
		this.server_id = server_id;
		this.server_pubkey = server_pubkey;
		this.seqno = seqno;
	}
}

class GetServersResponse {
	serialize(get_servers_response) {
		throw "TODO: implement GetServersResponse.serialize";
	}

	deserialize(buffer, offset) {
		var servers_response = new GetServersResponse();
		servers_response.server_pubkey = Pubkey.prototype.deserialize(buffer, offset);
		offset += 32;
		
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
			var seqno = U64.prototype.deserialize(buffer, offset);
			offset = seqno.offset;
			servers_response.servers.push(
				new ServerInfo(
					name,
					description,
					server_id,
					server_pubkey,
					seqno,
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
			this.request_id = Math.floor(Math.random() * 4294967296); // u32
			if(this.event_type == EVENT_TYPE_AUTH) {
				this.auth_event = event_data;
			} else if(this.event_type == EVENT_TYPE_CHALLENGE) {
				this.challenge_event = event_data;
			} else if(this.event_type == EVENT_TYPE_AUTH_RESP) {
				this.auth_resp = event_data;
			} else if(this.event_type == EVENT_TYPE_GET_SERVERS_EVENT) {
				this.get_servers_event = new GetServersEvent();
			} else if(this.event_type == EVENT_TYPE_CREATE_SERVER_EVENT) {
				this.create_server_event = event_data;
			} else if(this.event_type == EVENT_TYPE_DELETE_SERVER_EVENT) {
				this.delete_server_event = event_data;
			} else if(this.event_type == EVENT_TYPE_MODIFY_SERVER_EVENT) {
				this.modify_server_event = event_data;
			} else if(this.event_type == EVENT_TYPE_GET_CHANNELS_REQUEST) {
				this.get_channels_request = event_data;
			} else if(this.event_type == EVENT_TYPE_GET_CHANNELS_RESPONSE) {
				this.get_channels_response = event_data;
			} else if(this.event_type == EVENT_TYPE_ADD_CHANNEL_REQUEST) {
				this.add_channel_request = event_data;
			} else if(this.event_type == EVENT_TYPE_DELETE_CHANNEL_REQUEST) {
				this.delete_channel_request = event_data;
			} else if(this.event_type == EVENT_TYPE_MODIFY_CHANNEL_REQUEST) {
				this.modify_channel_request = event_data;
			} else if(this.event_type == EVENT_TYPE_GET_MEMBERS_REQUEST) {
				this.get_members_request = event_data;
			} else if(this.event_type == EVENT_TYPE_GET_MEMBERS_RESPONSE) {
				this.get_members_response = event_data;
			} else if(this.event_type == EVENT_TYPE_LIST_INVITES_REQUEST) {
				this.list_invites_request = event_data;
			} else if(this.event_type == EVENT_TYPE_SET_PROFILE_REQUEST) {
				this.set_profile_request = event_data;
			} else if(this.event_type == EVENT_TYPE_CREATE_INVITE_REQUEST) {
				this.create_invite_request = event_data;
			} else if(this.event_type == EVENT_TYPE_LIST_INVITES_RESPONSE) {
				this.list_invites_response = event_data;
			} else if(this.event_type == EVENT_TYPE_DELETE_INVITE_REQUEST) {
				this.delete_invite_request = event_data;
			} else {
				throw "Unknown event in Event.constructor type = " + event_type;
			}
		}
	}

	serialize(event) {
		var x;
		if(event.event_type == EVENT_TYPE_AUTH) {
			x = event.auth_event.serialize(event.auth_event);
		} else if(event.event_type == EVENT_TYPE_CHALLENGE) {
			x = event.challenge_event.serialize(event.challenge_event);
		} else if(event.event_type == EVENT_TYPE_AUTH_RESP) {
			x = event.auth_resp.serialize(event.auth_resp);
		} else if(event.event_type == EVENT_TYPE_GET_SERVERS_EVENT) {
			x = new Uint8Array(0);
		} else if(event.event_type == EVENT_TYPE_CREATE_SERVER_EVENT) {
			x = event.create_server_event.serialize(event.create_server_event);
		} else if(event.event_type == EVENT_TYPE_DELETE_SERVER_EVENT) {
			x = event.delete_server_event.serialize(event.delete_server_event);
		} else if(event.event_type == EVENT_TYPE_MODIFY_SERVER_EVENT) {
			x = event.modify_server_event.serialize(event.modify_server_event);
		} else if(event.event_type == EVENT_TYPE_GET_CHANNELS_REQUEST) {
			x = event.get_channels_request.serialize(event.get_channels_request);
		} else if(event.event_type == EVENT_TYPE_ADD_CHANNEL_REQUEST) {
			x = event.add_channel_request.serialize(event.add_channel_request);
		} else if(event.event_type == EVENT_TYPE_DELETE_CHANNEL_REQUEST) {
			x = event.delete_channel_request.serialize(event.delete_channel_request);
		} else if(event.event_type == EVENT_TYPE_MODIFY_CHANNEL_REQUEST) {
			x = event.modify_channel_request.serialize(event.modify_channel_request);
		} else if(event.event_type == EVENT_TYPE_GET_MEMBERS_REQUEST) {
			x = event.get_members_request.serialize(event.get_members_request);
		} else if(event.event_type == EVENT_TYPE_GET_MEMBERS_RESPONSE) {
			x = event.get_members_response.serialize(event.get_members_response);
		} else if(event.event_type == EVENT_TYPE_LIST_INVITES_REQUEST) {
			x = event.list_invites_request.serialize(event.list_invites_request);
		} else if(event.event_type == EVENT_TYPE_SET_PROFILE_REQUEST) {
			x = event.set_profile_request.serialize(event.set_profile_request);
		} else if(event.event_type == EVENT_TYPE_CREATE_INVITE_REQUEST) {
			x = event.create_invite_request.serialize(event.create_invite_request);
		} else if(event.event_type == EVENT_TYPE_LIST_INVITES_RESPONSE) {
			x = event.list_invites_response.serialize(event.list_invites_response);
		} else if(event.event_type == EVENT_TYPE_DELETE_INVITE_REQUEST) {
			x = event.delete_invite_request.serialize(event.delete_invite_request);
		} else {
			throw "Unknown event type in event.serialize = " + event.event_type;
		}

		var ret = new ArrayBuffer(x.length + FIRST_EVENT_DATA);
		var ret = new Uint8Array(ret);
		ret[0] = event.version;
		var t = U128.prototype.serialize(event.timestamp);
		for(var i=0; i<16; i++) {
			ret[i+1] = t[i];
		}
		var y = U32.prototype.serialize(event.request_id);
		for(var i=0; i<4; i++) {
			ret[i+17] = y[i];
		}
		ret[FIRST_EVENT_DATA-1] = event.event_type;
		for(var i=0; i<x.length; i++) {
			ret[i+FIRST_EVENT_DATA] = x[i];
		}
		return ret;

	}

	deserialize(buffer) {
		var event = new Event();
		event.version = buffer[0];
		event.timestamp = U128.prototype.deserialize(buffer, 1);
		event.request_id = U32.prototype.deserialize(buffer, 17).value;
		event.event_type = buffer[FIRST_EVENT_DATA-1];
		if(event.event_type == EVENT_TYPE_AUTH) {
			event.auth_event = AuthEvent
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_CHALLENGE) {
			event.challenge_event = ChallengeEvent
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_AUTH_RESP) {
			event.auth_resp = AuthResp
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_GET_SERVERS_EVENT) {
			event.get_servers = GetServersEvent
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_GET_SERVERS_RESPONSE) {
			event.get_servers_response = GetServersResponse
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_CREATE_SERVER_EVENT) {
			event.create_server_event = CreateServerEvent
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_DELETE_SERVER_EVENT) {
			event.delete_server_event = DeleteServerEvent
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_MODIFY_SERVER_EVENT) {
			event.modify_server_event = ModifyServerEvent
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_GET_CHANNELS_REQUEST) {
			event.get_channels_request = GetChannelsRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_GET_CHANNELS_RESPONSE) {
			event.get_channels_response = GetChannelsResponse
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_ADD_CHANNEL_REQUEST) {
			event.add_channel_response = AddChannelRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_DELETE_CHANNEL_REQUEST) {
			event.delete_channel_request = DeleteChannelRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_MODIFY_CHANNEL_REQUEST) {
			event.modify_channel_request = ModifyChannelRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_GET_MEMBERS_REQUEST) {
			event.get_members_request = GetMembersRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_GET_MEMBERS_RESPONSE) {
			event.get_members_response = GetMembersResponse
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_LIST_INVITES_REQUEST) {
			event.list_invites_request = ListInvitesRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_SET_PROFILE_REQUEST) {
			event.set_profile_request = SetProfileRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_CREATE_INVITE_REQUEST) {
			event.create_invite_request = CreateInviteRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_LIST_INVITES_RESPONSE) {
			event.list_invites_response = ListInvitesResponse
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_DELETE_INVITE_REQUEST) {
			event.delete_invite_request = DeleteInviteRequest
				.prototype
				.deserialize(buffer, FIRST_EVENT_DATA);
		} else if(event.event_type == EVENT_TYPE_ADD_CHANNEL_RESPONSE ||
			event.event_type == EVENT_TYPE_MODIFY_CHANNEL_RESPONSE ||
			event.event_type == EVENT_TYPE_DELETE_CHANNEL_RESPONSE){
			// for now we don't process these
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

