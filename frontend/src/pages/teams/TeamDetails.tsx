import React, { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';

interface Team {
  pid: string;
  name: string;
  description: string | null;
  owner_id: number;
  created_at: string;
  updated_at: string;
}

interface TeamMember {
  pid: string;
  user_id: number;
  role: string;
  user_name: string;
  user_email: string;
}

interface TeamInvitation {
  pid: string;
  email: string;
  role: string;
  status: string;
  created_at: string;
}

const TeamDetails: React.FC = () => {
  const { teamId } = useParams<{ teamId: string }>();
  const navigate = useNavigate();
  
  const [team, setTeam] = useState<Team | null>(null);
  const [members, setMembers] = useState<TeamMember[]>([]);
  const [invitations, setInvitations] = useState<TeamInvitation[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  
  const [isAdmin, setIsAdmin] = useState<boolean>(false);
  const [isOwner, setIsOwner] = useState<boolean>(false);
  
  const [inviteEmail, setInviteEmail] = useState<string>('');
  const [inviteRole, setInviteRole] = useState<string>('member');
  const [isInviting, setIsInviting] = useState<boolean>(false);
  const [inviteError, setInviteError] = useState<string | null>(null);

  useEffect(() => {
    const fetchTeamDetails = async () => {
      try {
        const token = localStorage.getItem('token');
        if (!token) {
          throw new Error('No authentication token found');
        }

        // Fetch team details
        const teamResponse = await fetch(`/api/teams/${teamId}`, {
          headers: {
            'Authorization': `Bearer ${token}`
          }
        });

        if (!teamResponse.ok) {
          throw new Error('Failed to fetch team details');
        }

        const teamData = await teamResponse.json();
        setTeam(teamData);

        // Fetch team members
        const membersResponse = await fetch(`/api/teams/${teamId}/members`, {
          headers: {
            'Authorization': `Bearer ${token}`
          }
        });

        if (!membersResponse.ok) {
          throw new Error('Failed to fetch team members');
        }

        const membersData = await membersResponse.json();
        setMembers(membersData);
        
        // Check if current user is admin or owner
        const currentUserData = membersData.find((member: TeamMember) => {
          return member.user_name === localStorage.getItem('userName');
        });
        
        if (currentUserData) {
          setIsAdmin(currentUserData.role === 'admin' || teamData.owner_id === currentUserData.user_id);
          setIsOwner(teamData.owner_id === currentUserData.user_id);
        }

        // Fetch invitations if user is admin or owner
        if (currentUserData && (currentUserData.role === 'admin' || teamData.owner_id === currentUserData.user_id)) {
          const invitationsResponse = await fetch(`/api/teams/${teamId}/invitations`, {
            headers: {
              'Authorization': `Bearer ${token}`
            }
          });

          if (invitationsResponse.ok) {
            const invitationsData = await invitationsResponse.json();
            setInvitations(invitationsData);
          }
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'An error occurred while fetching team details');
      } finally {
        setIsLoading(false);
      }
    };

    fetchTeamDetails();
  }, [teamId]);

  const handleInviteMember = async (e: React.FormEvent) => {
    e.preventDefault();
    setInviteError(null);
    setIsInviting(true);

    try {
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch(`/api/teams/${teamId}/invite`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          email: inviteEmail,
          role: inviteRole
        })
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to invite member');
      }

      const newInvitation = await response.json();
      setInvitations([...invitations, newInvitation]);
      setInviteEmail('');
      setInviteRole('member');
    } catch (err) {
      setInviteError(err instanceof Error ? err.message : 'An error occurred while inviting member');
    } finally {
      setIsInviting(false);
    }
  };

  const handleDeleteTeam = async () => {
    if (!window.confirm('Are you sure you want to delete this team? This action cannot be undone.')) {
      return;
    }

    try {
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch(`/api/teams/${teamId}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });

      if (!response.ok) {
        throw new Error('Failed to delete team');
      }

      navigate('/teams');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred while deleting team');
    }
  };

  const handleRemoveMember = async (memberPid: string) => {
    if (!window.confirm('Are you sure you want to remove this member from the team?')) {
      return;
    }

    try {
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch(`/api/teams/${teamId}/members/${memberPid}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });

      if (!response.ok) {
        throw new Error('Failed to remove member');
      }

      setMembers(members.filter(member => member.pid !== memberPid));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred while removing member');
    }
  };

  const handleUpdateMemberRole = async (memberPid: string, newRole: string) => {
    try {
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch(`/api/teams/${teamId}/members/${memberPid}/role`, {
        method: 'PUT',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          role: newRole
        })
      });

      if (!response.ok) {
        throw new Error('Failed to update member role');
      }

      const updatedMember = await response.json();
      setMembers(members.map(member => 
        member.pid === memberPid ? updatedMember : member
      ));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred while updating member role');
    }
  };

  if (isLoading) {
    return <div className="text-center py-10">Loading team details...</div>;
  }

  if (error) {
    return (
      <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4" role="alert">
        <span className="block sm:inline">{error}</span>
      </div>
    );
  }

  if (!team) {
    return <div className="text-center py-10">Team not found</div>;
  }

  return (
    <div>
      <div className="mb-8">
        <div className="flex justify-between items-center mb-4">
          <h1 className="text-2xl font-bold">{team.name}</h1>
          {isOwner && (
            <button 
              onClick={handleDeleteTeam}
              className="bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded"
            >
              Delete Team
            </button>
          )}
        </div>
        <p className="text-gray-600">{team.description || 'No description'}</p>
      </div>

      <div className="mb-8">
        <h2 className="text-xl font-semibold mb-4">Team Members</h2>
        <div className="bg-white shadow overflow-hidden sm:rounded-lg">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Name
                </th>
                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Email
                </th>
                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Role
                </th>
                {isAdmin && (
                  <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Actions
                  </th>
                )}
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {members.map((member) => (
                <tr key={member.pid}>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="text-sm font-medium text-gray-900">{member.user_name}</div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="text-sm text-gray-500">{member.user_email}</div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    {isOwner && member.role !== 'owner' ? (
                      <select 
                        value={member.role}
                        onChange={(e) => handleUpdateMemberRole(member.pid, e.target.value)}
                        className="text-sm text-gray-900 border rounded px-2 py-1"
                      >
                        <option value="member">Member</option>
                        <option value="admin">Admin</option>
                      </select>
                    ) : (
                      <span className="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-blue-100 text-blue-800">
                        {member.role}
                      </span>
                    )}
                  </td>
                  {isAdmin && (
                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                      {member.role !== 'owner' && (isOwner || (isAdmin && member.role !== 'admin')) && (
                        <button 
                          onClick={() => handleRemoveMember(member.pid)}
                          className="text-red-600 hover:text-red-900"
                        >
                          Remove
                        </button>
                      )}
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {isAdmin && (
        <div className="mb-8">
          <h2 className="text-xl font-semibold mb-4">Invite New Member</h2>
          <div className="bg-white p-6 rounded-lg shadow-md">
            {inviteError && (
              <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4" role="alert">
                <span className="block sm:inline">{inviteError}</span>
              </div>
            )}
            <form onSubmit={handleInviteMember} className="space-y-4">
              <div>
                <label htmlFor="inviteEmail" className="block text-sm font-medium text-gray-700">
                  Email Address
                </label>
                <input
                  type="email"
                  id="inviteEmail"
                  value={inviteEmail}
                  onChange={(e) => setInviteEmail(e.target.value)}
                  className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                  required
                />
              </div>
              <div>
                <label htmlFor="inviteRole" className="block text-sm font-medium text-gray-700">
                  Role
                </label>
                <select
                  id="inviteRole"
                  value={inviteRole}
                  onChange={(e) => setInviteRole(e.target.value)}
                  className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                >
                  <option value="member">Member</option>
                  {isOwner && <option value="admin">Admin</option>}
                </select>
              </div>
              <button
                type="submit"
                disabled={isInviting}
                className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
              >
                {isInviting ? 'Sending Invitation...' : 'Send Invitation'}
              </button>
            </form>
          </div>
        </div>
      )}

      {isAdmin && invitations.length > 0 && (
        <div className="mb-8">
          <h2 className="text-xl font-semibold mb-4">Pending Invitations</h2>
          <div className="bg-white shadow overflow-hidden sm:rounded-lg">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Email
                  </th>
                  <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Role
                  </th>
                  <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Date
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {invitations.map((invitation) => (
                  <tr key={invitation.pid}>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="text-sm text-gray-900">{invitation.email}</div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="text-sm text-gray-500">{invitation.role}</div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${
                        invitation.status === 'pending' ? 'bg-yellow-100 text-yellow-800' :
                        invitation.status === 'accepted' ? 'bg-green-100 text-green-800' :
                        'bg-red-100 text-red-800'
                      }`}>
                        {invitation.status}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {new Date(invitation.created_at).toLocaleDateString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
};

export default TeamDetails;
