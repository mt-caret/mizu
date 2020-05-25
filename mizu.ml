open SCaml

type action =
  | Post of bytes list * nat list
  | Poke of address
  | Register of bytes

(* Timestamps impose a total ordering on all messages, as Tezos should
 * guarantee strict monotonicity.
 * TODO: is this true? can we rely on this? *)
type message =
  { content : bytes
  ; timestamp : timestamp
  }

type user_data =
  { signed_prekey : bytes
  ; postal_box : message list
  ; pokes : address list
  }

type storage = (address, user_data) big_map

let is_empty : nat list -> bool = function
  | [] -> true
  | _ -> false
;;

let incr (n : nat) : nat = n +^ Nat 1

let post (add : bytes list) (remove : nat list) (storage : storage) =
  let sender = Global.get_sender () in
  let timestamp = Global.get_now () in
  let new_messages = List.map (fun content -> { content; timestamp }) add in
  let new_user_data =
    (* You can only post to your own postal box *)
    match BigMap.get sender storage with
    | None -> failwith "user is not registered"
    | Some user_data ->
      let _, remaining_indices, postal_box =
        (* We assume here that [remove] is sorted in ascending order,
         * so all elements are actually removed. *)
        List.fold_left
          (fun (index, rm_indices, accum) element ->
            match rm_indices with
            | [] -> incr index, [], element :: accum
            | x :: xs when index = x -> incr index, xs, accum
            | _ -> incr index, rm_indices, element :: accum)
          (Nat 0, remove, new_messages)
          user_data.postal_box
      in
      (* [remaining_indices] should be empty, which would not be the case
       * if [remove] was not given in ascending order or has elements
       * greater or equal to the length of [user_data.postal_box] *)
      assert (is_empty remaining_indices);
      { user_data with postal_box }
  in
  ([] : operation list), BigMap.update sender (Some new_user_data) storage
;;

let poke (address : address) (storage : storage) =
  (* Anybody can poke anybody else *)
  match BigMap.get address storage with
  | None -> failwith "invalid address"
  | Some user_data ->
    let new_user_data =
      { user_data with pokes = Global.get_sender () :: user_data.pokes }
    in
    ([] : operation list), BigMap.update address (Some new_user_data) storage
;;

let register (signed_prekey : bytes) (storage : storage) =
  let sender = Global.get_sender () in
  let new_user_data =
    (* create new [user_data] instance or update signed_prekey *)
    match BigMap.get sender storage with
    | None -> { signed_prekey; postal_box = []; pokes = [] }
    | Some user_data -> { user_data with signed_prekey }
  in
  ([] : operation list), BigMap.update sender (Some new_user_data) storage
;;

let[@entry] main action storage =
  match action with
  | Post (add, remove) -> post add remove storage
  | Poke address -> poke address storage
  | Register signed_prekey -> register signed_prekey storage
;;
